//! ComparisonStrategy — comparison operators and special forms.
//!
//! ```text
//! comparison ::= addition (comp_op addition)?
//!             | addition IS [NOT] NULL
//!             | addition [NOT] IN '(' expr (',' expr)* ')'
//!             | addition [NOT] LIKE pattern [ESCAPE char]
//!             | addition [NOT] BETWEEN low AND high
//! comp_op    ::= '=' | '!=' | '<>' | '<' | '>' | '<=' | '>='
//! ```
//!
//! Chains `ComparisonParser` implementations for special forms (IS, IN,
//! LIKE, BETWEEN), then falls through to standard comparison operators.
//! `NOT` prefix is handled here — wraps matched form in `UniOp(Not, ...)`.

use partiql_ast::ast;
use partiql_ast::ast::{BinOp, BinOpKind, UniOp, UniOpKind};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::comparison::between_parser::BetweenParser;
use super::comparison::in_parser::InParser;
use super::comparison::is_parser::IsParser;
use super::comparison::like_parser::LikeParser;
use super::comparison::ComparisonParser;
use super::{ExprStrategy, StrategyContext};
use crate::keyword::{kw, lit};
use crate::whitespace::{ws, ws0};

pub struct ComparisonStrategy {
    parsers: Vec<Box<dyn ComparisonParser>>,
}

impl ComparisonStrategy {
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(IsParser),
                Box::new(InParser),
                Box::new(LikeParser),
                Box::new(BetweenParser),
            ],
        }
    }
}

impl Default for ComparisonStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ExprStrategy for ComparisonStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let left = ctx.parse_next_level(input)?;
        let _ = ws0(input);

        // Try each comparison special form (IS, IN, LIKE, BETWEEN)
        for parser in &self.parsers {
            let checkpoint = *input;
            match parser.parse(input, ctx, &left) {
                Ok(expr) => return Ok(expr),
                Err(winnow::error::ErrMode::Backtrack(_)) => {
                    *input = checkpoint;
                }
                Err(e) => return Err(e),
            }
        }

        // NOT IN / NOT LIKE / NOT BETWEEN
        let checkpoint = *input;
        if (kw("NOT"), ws).parse_next(input).is_ok() {
            for parser in &self.parsers {
                let inner_checkpoint = *input;
                match parser.parse(input, ctx, &left) {
                    Ok(expr) => {
                        return Ok(ast::Expr::UniOp(ctx.node(UniOp {
                            kind: UniOpKind::Not,
                            expr: Box::new(expr),
                        })));
                    }
                    Err(winnow::error::ErrMode::Backtrack(_)) => {
                        *input = inner_checkpoint;
                    }
                    Err(e) => return Err(e),
                }
            }
            *input = checkpoint;
        }

        // Comparison operators: = != <> < > <= >=
        // Note: `<` must not match `<<` (bag open), `>` must not match `>>` (bag close)
        if let Ok(kind) = alt((
            lit("!=").map(|_| BinOpKind::Ne),
            lit("<>").map(|_| BinOpKind::Ne),
            lit("<=").map(|_| BinOpKind::Lte),
            lit(">=").map(|_| BinOpKind::Gte),
            lit("=").map(|_| BinOpKind::Eq),
            parse_single_lt.map(|_| BinOpKind::Lt),
            parse_single_gt.map(|_| BinOpKind::Gt),
        ))
        .parse_next(input)
        {
            let _ = ws0(input);
            let right = ctx.parse_next_level(input)?;
            return Ok(ast::Expr::BinOp(ctx.node(BinOp {
                kind,
                lhs: Box::new(left),
                rhs: Box::new(right),
            })));
        }

        Ok(left)
    }

    fn name(&self) -> &str {
        "Comparison"
    }
}

/// Match `<` but NOT `<<` (bag open) or `<=` or `<>`.
#[inline]
fn parse_single_lt<'a>(input: &mut &'a str) -> PResult<&'a str> {
    let checkpoint = *input;
    let matched = lit("<").parse_next(input)?;
    if let Some(&b) = input.as_bytes().first() {
        if b == b'<' || b == b'=' || b == b'>' {
            *input = checkpoint;
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
    }
    Ok(matched)
}

/// Match `>` but NOT `>>` (bag close) or `>=`.
#[inline]
fn parse_single_gt<'a>(input: &mut &'a str) -> PResult<&'a str> {
    let checkpoint = *input;
    let matched = lit(">").parse_next(input)?;
    if let Some(&b) = input.as_bytes().first() {
        if b == b'>' || b == b'=' {
            *input = checkpoint;
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
    }
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use crate::expr::ExprChain;
    use partiql_ast::ast;
    use partiql_ast::ast::{BinOpKind, Lit};

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    fn assert_comparison(expr: &ast::Expr, expected_kind: BinOpKind) {
        match expr {
            ast::Expr::BinOp(n) => {
                assert_eq!(n.node.kind, expected_kind);
                assert!(matches!(
                    &*n.node.lhs,
                    ast::Expr::VarRef(v) if v.node.name.value == "a"
                ));
                assert!(matches!(
                    &*n.node.rhs,
                    ast::Expr::Lit(lit) if matches!(lit.node, Lit::Int64Lit(1))
                ));
            }
            _ => panic!("expected BinOp"),
        }
    }

    #[test]
    fn test_eq() {
        let expr = parse("a = 1");
        assert_comparison(&expr, BinOpKind::Eq);
    }

    #[test]
    fn test_ne_bang() {
        let expr = parse("a != 1");
        assert_comparison(&expr, BinOpKind::Ne);
    }

    #[test]
    fn test_ne_diamond() {
        let expr = parse("a <> 1");
        assert_comparison(&expr, BinOpKind::Ne);
    }

    #[test]
    fn test_lt() {
        let expr = parse("a < 1");
        assert_comparison(&expr, BinOpKind::Lt);
    }

    #[test]
    fn test_gt() {
        let expr = parse("a > 1");
        assert_comparison(&expr, BinOpKind::Gt);
    }

    #[test]
    fn test_lte() {
        let expr = parse("a <= 1");
        assert_comparison(&expr, BinOpKind::Lte);
    }

    #[test]
    fn test_gte() {
        let expr = parse("a >= 1");
        assert_comparison(&expr, BinOpKind::Gte);
    }
}
