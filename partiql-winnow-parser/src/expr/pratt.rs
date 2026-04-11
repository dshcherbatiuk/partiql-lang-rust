//! Pratt parser — single-loop expression engine with binding power.
//!
//! Replaces the 9-level `ExprChain` recursive descent with a flat loop.
//! For a literal `1`: 1 call instead of 9.
//! For `a = 1 AND b = 2`: ~7 calls instead of ~45.

use partiql_ast::ast;
use partiql_ast::ast::{
    BinOp, BinOpKind, CaseSensitivity, Lit, Path, PathExpr, PathStep, ScopeQualifier,
    SymbolPrimitive, UniOp, UniOpKind, VarRef,
};
use winnow::prelude::*;

use crate::expr::comparison::between_parser::BetweenParser;
use crate::expr::comparison::in_parser::InParser;
use crate::expr::comparison::is_parser::IsParser;
use crate::expr::comparison::like_parser::LikeParser;
use crate::expr::comparison::ComparisonParser;
use crate::expr::primary_strategy::PrimaryStrategy;
use crate::expr::StrategyContext;
use crate::identifier;
use crate::keyword::{ch, kw};
use crate::parse_context::ParseContext;
use crate::whitespace::{ws, ws0};

/// Pratt expression parser — stateless, created once, reused.
pub struct PrattParser {
    primary: PrimaryStrategy,
}

impl PrattParser {
    pub fn new() -> Self {
        Self {
            primary: PrimaryStrategy::new(),
        }
    }

    /// Parse a full expression from minimum binding power 0.
    #[inline]
    pub fn parse_expr<'a>(
        &self,
        input: &mut &'a str,
        pctx: &ParseContext,
    ) -> PResult<ast::Expr> {
        self.parse_bp(input, pctx, 0)
    }

    /// Core Pratt loop — parse expression with minimum binding power.
    pub(crate) fn parse_bp<'a>(
        &self,
        input: &mut &'a str,
        pctx: &ParseContext,
        min_bp: u8,
    ) -> PResult<ast::Expr> {
        let _ = ws0(input);

        // Prefix: NOT, -, +
        let mut lhs = match input.as_bytes().first() {
            Some(b'-') => {
                let checkpoint = *input;
                ch('-').parse_next(input)?;
                // Check it's not -- (comment) or a number following an operator
                let _ = ws0(input);
                match self.parse_bp(input, pctx, PREFIX_BP) {
                    Ok(rhs) => ast::Expr::UniOp(pctx.node(UniOp {
                        kind: UniOpKind::Neg,
                        expr: Box::new(rhs),
                    })),
                    Err(_) => {
                        *input = checkpoint;
                        self.parse_primary(input, pctx)?
                    }
                }
            }
            Some(b'+') => {
                ch('+').parse_next(input)?;
                let _ = ws0(input);
                let rhs = self.parse_bp(input, pctx, PREFIX_BP)?;
                ast::Expr::UniOp(pctx.node(UniOp {
                    kind: UniOpKind::Pos,
                    expr: Box::new(rhs),
                }))
            }
            _ => {
                // Check for NOT prefix
                let checkpoint = *input;
                if (kw("NOT"), ws).parse_next(input).is_ok() {
                    let rhs = self.parse_bp(input, pctx, NOT_PREFIX_BP)?;
                    ast::Expr::UniOp(pctx.node(UniOp {
                        kind: UniOpKind::Not,
                        expr: Box::new(rhs),
                    }))
                } else {
                    *input = checkpoint;
                    self.parse_primary(input, pctx)?
                }
            }
        };

        // Infix + postfix loop
        loop {
            let _ = ws0(input);

            // Postfix: . [] (highest binding power)
            if let Some(b'.' | b'[') = input.as_bytes().first() {
                if let Some(expr) = self.try_postfix(input, pctx, &lhs)? {
                    lhs = expr;
                    continue;
                }
            }

            // Try infix operator
            let checkpoint = *input;
            if let Some((op, l_bp, r_bp)) = self.peek_infix(input) {
                if l_bp < min_bp {
                    *input = checkpoint;
                    break;
                }
                // Consume operator
                self.consume_infix(input, &op)?;

                // Special forms that need custom parsing
                match op {
                    InfixOp::Is => {
                        let is_parser = IsParser;
                        let ctx = self.make_ctx(pctx);
                        match is_parser.parse(input, &ctx, &lhs) {
                            Ok(expr) => {
                                lhs = expr;
                                continue;
                            }
                            Err(_) => {
                                *input = checkpoint;
                                break;
                            }
                        }
                    }
                    InfixOp::In => {
                        let in_parser = InParser;
                        let ctx = self.make_ctx(pctx);
                        match in_parser.parse(input, &ctx, &lhs) {
                            Ok(expr) => {
                                lhs = expr;
                                continue;
                            }
                            Err(_) => {
                                *input = checkpoint;
                                break;
                            }
                        }
                    }
                    InfixOp::Like => {
                        let like_parser = LikeParser;
                        let ctx = self.make_ctx(pctx);
                        match like_parser.parse(input, &ctx, &lhs) {
                            Ok(expr) => {
                                lhs = expr;
                                continue;
                            }
                            Err(_) => {
                                *input = checkpoint;
                                break;
                            }
                        }
                    }
                    InfixOp::Between => {
                        let between_parser = BetweenParser;
                        let ctx = self.make_ctx(pctx);
                        match between_parser.parse(input, &ctx, &lhs) {
                            Ok(expr) => {
                                lhs = expr;
                                continue;
                            }
                            Err(_) => {
                                *input = checkpoint;
                                break;
                            }
                        }
                    }
                    InfixOp::Not => {
                        // NOT IN / NOT LIKE / NOT BETWEEN
                        let _ = ws0(input);
                        let inner_checkpoint = *input;
                        let ctx = self.make_ctx(pctx);

                        if let Ok(expr) = InParser.parse(input, &ctx, &lhs) {
                            lhs = ast::Expr::UniOp(pctx.node(UniOp {
                                kind: UniOpKind::Not,
                                expr: Box::new(expr),
                            }));
                            continue;
                        }
                        *input = inner_checkpoint;
                        if let Ok(expr) = LikeParser.parse(input, &ctx, &lhs) {
                            lhs = ast::Expr::UniOp(pctx.node(UniOp {
                                kind: UniOpKind::Not,
                                expr: Box::new(expr),
                            }));
                            continue;
                        }
                        *input = inner_checkpoint;
                        if let Ok(expr) = BetweenParser.parse(input, &ctx, &lhs) {
                            lhs = ast::Expr::UniOp(pctx.node(UniOp {
                                kind: UniOpKind::Not,
                                expr: Box::new(expr),
                            }));
                            continue;
                        }
                        *input = checkpoint;
                        break;
                    }
                    InfixOp::BinOp(kind) => {
                        let _ = ws0(input);
                        let rhs = self.parse_bp(input, pctx, r_bp)?;
                        lhs = ast::Expr::BinOp(pctx.node(BinOp {
                            kind,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        }));
                    }
                }
            } else {
                break;
            }
        }

        Ok(lhs)
    }

    /// Parse primary expression — delegates to PrimaryStrategy.
    #[inline]
    fn parse_primary<'a>(
        &self,
        input: &mut &'a str,
        pctx: &ParseContext,
    ) -> PResult<ast::Expr> {
        let ctx = self.make_ctx(pctx);
        self.primary.parse_primary(input, &ctx)
    }

    /// Try postfix operators: `.field`, `[index]`
    fn try_postfix<'a>(
        &self,
        input: &mut &'a str,
        pctx: &ParseContext,
        lhs: &ast::Expr,
    ) -> Result<Option<ast::Expr>, winnow::error::ErrMode<winnow::error::ContextError>> {
        let mut root = lhs.clone();
        let mut steps = Vec::new();

        // Collect existing path steps if lhs is already a Path
        if let ast::Expr::Path(path_node) = root {
            root = *path_node.node.root;
            steps = path_node.node.steps;
        }

        let mut matched = false;
        loop {
            if ch('.').parse_next(input).is_ok() {
                let _ = ws0(input);
                let field = identifier::identifier(input)?;
                // Wrap field name in VarRef (matching LALRPOP shape) so that
                // downstream `name_resolver::infer_alias` can recover the field
                // name as the projection alias instead of falling back to `_N`.
                steps.push(PathStep::PathProject(PathExpr {
                    index: Box::new(ast::Expr::VarRef(pctx.node(VarRef {
                        name: SymbolPrimitive {
                            value: field.to_string(),
                            case: CaseSensitivity::CaseInsensitive,
                        },
                        qualifier: ScopeQualifier::Unqualified,
                    }))),
                }));
                matched = true;
                let _ = ws0(input);
            } else if ch('[').parse_next(input).is_ok() {
                let _ = ws0(input);
                let idx = self.parse_bp(input, pctx, 0)?;
                let _ = ws0(input);
                ch(']').parse_next(input)?;
                steps.push(PathStep::PathIndex(PathExpr {
                    index: Box::new(idx),
                }));
                matched = true;
                let _ = ws0(input);
            } else {
                break;
            }
        }

        if matched {
            Ok(Some(ast::Expr::Path(pctx.node(Path {
                root: Box::new(root),
                steps,
            }))))
        } else {
            Ok(None)
        }
    }

    /// Peek at the next infix operator without consuming.
    #[inline]
    fn peek_infix(&self, input: &mut &str) -> Option<(InfixOp, u8, u8)> {
        let bytes = input.as_bytes();
        match bytes.first()? {
            b'=' => Some((InfixOp::BinOp(BinOpKind::Eq), 5, 6)),
            b'!' if bytes.get(1) == Some(&b'=') => Some((InfixOp::BinOp(BinOpKind::Ne), 5, 6)),
            b'<' => {
                match bytes.get(1) {
                    Some(b'=') => Some((InfixOp::BinOp(BinOpKind::Lte), 5, 6)),
                    Some(b'>') => Some((InfixOp::BinOp(BinOpKind::Ne), 5, 6)),
                    Some(b'<') => None, // << bag
                    _ => Some((InfixOp::BinOp(BinOpKind::Lt), 5, 6)),
                }
            }
            b'>' => {
                match bytes.get(1) {
                    Some(b'=') => Some((InfixOp::BinOp(BinOpKind::Gte), 5, 6)),
                    Some(b'>') => None, // >> bag close
                    _ => Some((InfixOp::BinOp(BinOpKind::Gt), 5, 6)),
                }
            }
            b'+' => Some((InfixOp::BinOp(BinOpKind::Add), 7, 8)),
            b'-' => Some((InfixOp::BinOp(BinOpKind::Sub), 7, 8)),
            b'*' => Some((InfixOp::BinOp(BinOpKind::Mul), 9, 10)),
            b'/' => Some((InfixOp::BinOp(BinOpKind::Div), 9, 10)),
            b'%' => Some((InfixOp::BinOp(BinOpKind::Mod), 9, 10)),
            b'|' if bytes.get(1) == Some(&b'|') => {
                Some((InfixOp::BinOp(BinOpKind::Concat), 7, 8))
            }
            b'A' | b'a' => {
                // AND
                if bytes.len() >= 3
                    && bytes[..3].eq_ignore_ascii_case(b"AND")
                    && bytes.get(3).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    Some((InfixOp::BinOp(BinOpKind::And), 3, 4))
                } else {
                    None
                }
            }
            b'O' | b'o' => {
                // OR
                if bytes.len() >= 2
                    && bytes[..2].eq_ignore_ascii_case(b"OR")
                    && bytes.get(2).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    Some((InfixOp::BinOp(BinOpKind::Or), 1, 2))
                } else {
                    None
                }
            }
            b'I' | b'i' => {
                // IS, IN
                if bytes.len() >= 2
                    && bytes[..2].eq_ignore_ascii_case(b"IS")
                    && bytes.get(2).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    Some((InfixOp::Is, 5, 6))
                } else if bytes.len() >= 2
                    && bytes[..2].eq_ignore_ascii_case(b"IN")
                    && bytes.get(2).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    Some((InfixOp::In, 5, 6))
                } else {
                    None
                }
            }
            b'L' | b'l' => {
                // LIKE
                if bytes.len() >= 4
                    && bytes[..4].eq_ignore_ascii_case(b"LIKE")
                    && bytes.get(4).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    Some((InfixOp::Like, 5, 6))
                } else {
                    None
                }
            }
            b'B' | b'b' => {
                // BETWEEN
                if bytes.len() >= 7
                    && bytes[..7].eq_ignore_ascii_case(b"BETWEEN")
                    && bytes.get(7).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    Some((InfixOp::Between, 5, 6))
                } else {
                    None
                }
            }
            b'N' | b'n' => {
                // NOT (infix: NOT IN, NOT LIKE, NOT BETWEEN)
                if bytes.len() >= 3
                    && bytes[..3].eq_ignore_ascii_case(b"NOT")
                    && bytes.get(3).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    Some((InfixOp::Not, 5, 6))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Consume the infix operator token.
    #[inline]
    fn consume_infix<'a>(
        &self,
        input: &mut &'a str,
        op: &InfixOp,
    ) -> PResult<()> {
        match op {
            InfixOp::BinOp(kind) => match kind {
                BinOpKind::Ne => {
                    if input.starts_with("!=") {
                        *input = &input[2..];
                    } else {
                        // <>
                        *input = &input[2..];
                    }
                }
                BinOpKind::Lte | BinOpKind::Gte => {
                    *input = &input[2..];
                }
                BinOpKind::Concat => {
                    *input = &input[2..];
                }
                BinOpKind::And => {
                    *input = &input[3..];
                }
                BinOpKind::Or => {
                    *input = &input[2..];
                }
                _ => {
                    // Single char: = < > + - * / %
                    *input = &input[1..];
                }
            },
            InfixOp::Is => {
                // IS is peeked but consumed by IsParser
                // Don't consume here — IsParser handles it
            }
            InfixOp::In => {
                // IN consumed by InParser
            }
            InfixOp::Like => {
                // LIKE consumed by LikeParser
            }
            InfixOp::Between => {
                // BETWEEN consumed by BetweenParser
            }
            InfixOp::Not => {
                *input = &input[3..]; // consume NOT
            }
        }
        Ok(())
    }

    /// Create a StrategyContext for delegating to ComparisonParsers and PrimaryStrategy.
    #[inline]
    fn make_ctx<'c>(&'c self, pctx: &'c ParseContext) -> StrategyContext<'c> {
        StrategyContext::new(self, pctx)
    }
}

impl Default for PrattParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Prefix binding power for unary -, +
const PREFIX_BP: u8 = 11;

/// Prefix binding power for NOT — lower than comparison(5) so NOT captures full condition
const NOT_PREFIX_BP: u8 = 3;

/// Infix operator classification
#[derive(Debug)]
enum InfixOp {
    BinOp(BinOpKind),
    Is,
    In,
    Like,
    Between,
    Not, // NOT IN, NOT LIKE, NOT BETWEEN
}
