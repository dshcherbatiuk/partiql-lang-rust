//! PrimaryStrategy — highest precedence, bottom of the expression chain.
//!
//! Chains literal strategies from `literal/` with expression-level
//! strategies (parenthesized expressions, identifiers).
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │ PrimaryStrategy chain                    │
//! │                                          │
//! │  literal/string_strategy                 │
//! │  literal/number_strategy                 │
//! │  literal/null_strategy                   │
//! │  literal/boolean_strategy                │
//! │  ParenExprStrategy       (in expr/)      │
//! │  IdentifierStrategy      (in expr/)      │
//! └──────────────────────────────────────────┘
//! ```

use partiql_ast::ast;
use partiql_ast::ast::{
    Call, CallAgg, CallArg, CaseSensitivity, ScopeQualifier, SymbolPrimitive, VarRef,
};
use winnow::prelude::*;

use super::StrategyContext;
use crate::identifier;
use crate::keyword::ch;
use crate::literal::bag_strategy::BagConstructorStrategy;
use crate::literal::boolean_strategy::BooleanLiteralStrategy;
use crate::literal::case_strategy::CaseExprStrategy;
use crate::literal::list_strategy::ListConstructorStrategy;
use crate::literal::null_strategy::NullMissingStrategy;
use crate::literal::number_strategy::NumericLiteralStrategy;
use crate::literal::string_strategy::StringLiteralStrategy;
use crate::literal::struct_strategy::StructConstructorStrategy;
use crate::literal::LiteralStrategy;
use crate::whitespace::ws0;

pub struct PrimaryStrategy {
    literal_strategies: Vec<Box<dyn LiteralStrategy>>,
}

impl PrimaryStrategy {
    pub fn new() -> Self {
        Self {
            literal_strategies: vec![
                Box::new(BagConstructorStrategy),
                Box::new(ListConstructorStrategy),
                Box::new(StructConstructorStrategy),
                Box::new(CaseExprStrategy),
                Box::new(StringLiteralStrategy),
                Box::new(NumericLiteralStrategy),
                Box::new(NullMissingStrategy),
                Box::new(BooleanLiteralStrategy),
            ],
        }
    }
}

impl Default for PrimaryStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimaryStrategy {
    /// Parse a primary expression — called by PrattParser directly.
    #[inline]
    pub fn parse_primary<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let _ = ws0(input);

        // First-char dispatch — avoids iterating 8 strategies for common cases
        match input.as_bytes().first() {
            Some(b'\'') => {
                return self.literal_strategies[4].parse(input, ctx); // StringLiteralStrategy
            }
            Some(b'0'..=b'9') => {
                return self.literal_strategies[5].parse(input, ctx); // NumericLiteralStrategy
            }
            Some(b'[') => {
                return self.literal_strategies[1].parse(input, ctx); // ListConstructorStrategy
            }
            Some(b'{') => {
                return self.literal_strategies[2].parse(input, ctx); // StructConstructorStrategy
            }
            Some(b'(') => {
                return parse_paren_expr(input, ctx);
            }
            Some(b'<') => {
                // << bag >> or fall through
                let checkpoint = *input;
                if let Ok(expr) = self.literal_strategies[0].parse(input, ctx) {
                    return Ok(expr); // BagConstructorStrategy
                }
                *input = checkpoint;
            }
            Some(b'"') => {
                // Double-quoted = case-sensitive identifier
                return parse_identifier_or_call(input, ctx);
            }
            _ => {}
        }

        // Keywords: CASE, NULL, MISSING, TRUE, FALSE — or identifier/function call
        // Peek at keyword without consuming
        if let Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') = input.as_bytes().first() {
            // Try CASE first (starts with 'C'/'c')
            if input.len() >= 4 {
                let prefix = &input[..4];
                if prefix.eq_ignore_ascii_case("CASE")
                    && input.as_bytes().get(4).map_or(true, |b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    let checkpoint = *input;
                    if let Ok(expr) = self.literal_strategies[3].parse(input, ctx) {
                        return Ok(expr); // CaseExprStrategy
                    }
                    *input = checkpoint;
                }
            }

            // Try null/missing/true/false via keyword check (no allocation)
            let checkpoint = *input;
            if let Ok(expr) = self.literal_strategies[6].parse(input, ctx) {
                return Ok(expr); // NullMissingStrategy
            }
            *input = checkpoint;
            if let Ok(expr) = self.literal_strategies[7].parse(input, ctx) {
                return Ok(expr); // BooleanLiteralStrategy
            }
            *input = checkpoint;
        }

        // Function call or identifier (catches anything that looks like a name)
        parse_identifier_or_call(input, ctx)
    }

}

fn parse_paren_expr<'a>(input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
    let _ = '('.parse_next(input)?;
    let _ = ws0(input);
    let inner = ctx.parse_expr(input)?;
    let _ = ws0(input);
    let _ = ')'.parse_next(input)?;
    Ok(inner)
}

/// Parse identifier — if followed by `(`, parse as function call.
/// Double-quoted identifiers produce `CaseSensitive` VarRef.
fn parse_identifier_or_call<'a>(
    input: &mut &'a str,
    ctx: &StrategyContext<'_>,
) -> PResult<ast::Expr> {
    let (name, is_quoted) = identifier::identifier_with_case(input)?;
    let case = if is_quoted {
        CaseSensitivity::CaseSensitive
    } else {
        CaseSensitivity::CaseInsensitive
    };
    let _ = ws0(input);

    // Function call: name(args...)
    if ch('(').parse_next(input).is_ok() {
        let _ = ws0(input);

        let mut args = Vec::new();

        // Empty args: name()
        if ch(')').parse_next(input).is_ok() {
            return Ok(make_call(ctx, name, args));
        }

        // Star arg: COUNT(*)
        if ch('*').parse_next(input).is_ok() {
            args.push(ctx.node(CallArg::Star()));
            let _ = ws0(input);
            ch(')').parse_next(input)?;
            return Ok(make_call(ctx, name, args));
        }

        // Positional args: name(expr, expr, ...)
        loop {
            let _ = ws0(input);
            let expr = ctx.parse_expr(input)?;
            args.push(ctx.node(CallArg::Positional(Box::new(expr))));
            let _ = ws0(input);
            if ch(',').parse_next(input).is_err() {
                break;
            }
        }
        let _ = ws0(input);
        ch(')').parse_next(input)?;

        return Ok(make_call(ctx, name, args));
    }

/// Known SQL aggregate functions — produce CallAgg instead of Call.
#[inline]
fn is_aggregate(name: &str) -> bool {
    matches!(
        name.as_bytes(),
        [b'C' | b'c', b'O' | b'o', b'U' | b'u', b'N' | b'n', b'T' | b't']
        | [b'S' | b's', b'U' | b'u', b'M' | b'm']
        | [b'A' | b'a', b'V' | b'v', b'G' | b'g']
        | [b'M' | b'm', b'I' | b'i', b'N' | b'n']
        | [b'M' | b'm', b'A' | b'a', b'X' | b'x']
    )
}

/// Create Call or CallAgg based on function name.
#[inline]
fn make_call(
    ctx: &StrategyContext<'_>,
    name: &str,
    args: Vec<ast::AstNode<CallArg>>,
) -> ast::Expr {
    let func_name = SymbolPrimitive {
        value: name.to_string(),
        case: CaseSensitivity::CaseInsensitive,
    };
    if is_aggregate(name) {
        ast::Expr::CallAgg(ctx.node(CallAgg { func_name, args }))
    } else {
        ast::Expr::Call(ctx.node(Call { func_name, args }))
    }
}

    Ok(ast::Expr::VarRef(ctx.node(VarRef {
        name: SymbolPrimitive { value: name.to_string(), case },
        qualifier: ScopeQualifier::Unqualified,
    })))
}

#[cfg(test)]
mod tests {
    use partiql_ast::ast::{CallArg, Lit};

    use crate::expr::ExprChain;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let pctx = crate::parse_context::ParseContext::new();
        let mut i = input;
        chain.parse_expr(&mut i, &pctx).expect("parse failed")
    }

    use partiql_ast::ast;

    #[test]
    fn test_integer() {
        assert!(matches!(
            parse("42"),
            ast::Expr::Lit(n) if matches!(n.node, Lit::Int64Lit(42))
        ));
    }

    #[test]
    fn test_sql_string() {
        assert!(matches!(
            &parse("'hello'"),
            ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s == "hello")
        ));
    }

    #[test]
    fn test_quoted_identifier() {
        // Double-quoted strings are case-sensitive identifiers, not string literals
        assert!(matches!(
            &parse(r#""hello""#),
            ast::Expr::VarRef(n) if n.node.name.value == "hello"
                && n.node.name.case == ast::CaseSensitivity::CaseSensitive
        ));
    }

    #[test]
    fn test_bool_true() {
        assert!(matches!(
            parse("true"),
            ast::Expr::Lit(n) if matches!(n.node, Lit::BoolLit(true))
        ));
    }

    #[test]
    fn test_bool_false_uppercase() {
        assert!(matches!(
            parse("FALSE"),
            ast::Expr::Lit(n) if matches!(n.node, Lit::BoolLit(false))
        ));
    }

    #[test]
    fn test_bool_word_boundary() {
        assert!(matches!(
            &parse("trueish"),
            ast::Expr::VarRef(n) if n.node.name.value == "trueish"
        ));
    }

    #[test]
    fn test_null() {
        assert!(matches!(
            parse("null"),
            ast::Expr::Lit(n) if matches!(n.node, Lit::Null)
        ));
    }

    #[test]
    fn test_missing() {
        assert!(matches!(
            parse("MISSING"),
            ast::Expr::Lit(n) if matches!(n.node, Lit::Missing)
        ));
    }

    #[test]
    fn test_identifier() {
        assert!(matches!(
            &parse("users"),
            ast::Expr::VarRef(n) if n.node.name.value == "users"
        ));
    }

    #[test]
    fn test_parenthesized() {
        assert!(matches!(
            parse("(42)"),
            ast::Expr::Lit(n) if matches!(n.node, Lit::Int64Lit(42))
        ));
    }

    #[test]
    fn test_decimal() {
        assert!(matches!(
            parse("3.14"),
            ast::Expr::Lit(n) if matches!(n.node, Lit::DecimalLit(_))
        ));
    }

    // ── Expression chain tests (all strategies) ──────────────

    #[test]
    fn test_addition() {
        let e = parse("1 + 2");
        assert!(matches!(
            &e,
            ast::Expr::BinOp(n) if n.node.kind == ast::BinOpKind::Add
        ));
    }

    #[test]
    fn test_subtraction() {
        let e = parse("5 - 3");
        assert!(matches!(
            &e,
            ast::Expr::BinOp(n) if n.node.kind == ast::BinOpKind::Sub
        ));
    }

    #[test]
    fn test_multiplication() {
        let e = parse("2 * 3");
        assert!(matches!(
            &e,
            ast::Expr::BinOp(n) if n.node.kind == ast::BinOpKind::Mul
        ));
    }

    #[test]
    fn test_comparison_eq() {
        let e = parse("a = 1");
        assert!(matches!(
            &e,
            ast::Expr::BinOp(n) if n.node.kind == ast::BinOpKind::Eq
        ));
    }

    #[test]
    fn test_comparison_neq() {
        let e = parse("a != 1");
        assert!(matches!(
            &e,
            ast::Expr::BinOp(n) if n.node.kind == ast::BinOpKind::Ne
        ));
    }

    #[test]
    fn test_and() {
        let e = parse("a = 1 AND b = 2");
        assert!(matches!(
            &e,
            ast::Expr::BinOp(n) if n.node.kind == ast::BinOpKind::And
        ));
    }

    #[test]
    fn test_or() {
        let e = parse("a = 1 OR b = 2");
        assert!(matches!(
            &e,
            ast::Expr::BinOp(n) if n.node.kind == ast::BinOpKind::Or
        ));
    }

    #[test]
    fn test_not() {
        let e = parse("NOT true");
        assert!(matches!(
            &e,
            ast::Expr::UniOp(n) if n.node.kind == ast::UniOpKind::Not
        ));
    }

    /// Negative integer literals are parsed as `UniOp(Neg, Int64Lit(n))`
    /// (LALRPOP behaves the same way). Downstream consumers — the logical
    /// planner / evaluator — fold this at runtime via `EvalOpUnary::Neg`.
    /// Any AST→value direct conversion (e.g. FDE's Ion-native DML path)
    /// must therefore handle the unfolded `UniOp(Neg, Lit)` shape.
    #[test]
    fn test_negative_integer_literal() {
        let e = parse("-1");
        match &e {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, ast::UniOpKind::Neg);
                match &*n.node.expr {
                    ast::Expr::Lit(lit) => assert!(
                        matches!(lit.node, Lit::Int64Lit(1)),
                        "expected Int64Lit(1), got {:?}",
                        lit.node
                    ),
                    other => panic!("expected Lit inside UniOp, got {:?}", other),
                }
            }
            other => panic!("expected UniOp(Neg, Lit), got {:?}", other),
        }
    }

    #[test]
    fn test_negative_decimal_literal() {
        let e = parse("-3.14");
        match &e {
            ast::Expr::UniOp(n) => {
                assert_eq!(n.node.kind, ast::UniOpKind::Neg);
                assert!(matches!(*n.node.expr, ast::Expr::Lit(_)));
            }
            other => panic!("expected UniOp(Neg, Lit), got {:?}", other),
        }
    }

    #[test]
    fn test_path_dot() {
        let e = parse("a.b");
        assert!(matches!(&e, ast::Expr::Path(_)));
    }

    #[test]
    fn test_path_bracket() {
        let e = parse("a[0]");
        assert!(matches!(&e, ast::Expr::Path(_)));
    }

    /// `u.*` is the PathUnpivot step — it emits all fields of `u` as a
    /// tuple splat. LALRPOP supports it; the winnow parser must produce
    /// the same shape so the logical planner can lower both consistently.
    #[test]
    fn test_path_dot_star_unpivot() {
        let e = parse("u.*");
        match &e {
            ast::Expr::Path(p) => {
                assert!(matches!(*p.node.root, ast::Expr::VarRef(_)));
                assert_eq!(p.node.steps.len(), 1);
                assert!(matches!(p.node.steps[0], ast::PathStep::PathUnpivot));
            }
            other => panic!("expected Path with PathUnpivot step, got {:?}", other),
        }
    }

    /// `a[*]` is the PathForEach step — iterate over all elements of a list.
    #[test]
    fn test_path_bracket_star_for_each() {
        let e = parse("a[*]");
        match &e {
            ast::Expr::Path(p) => {
                assert_eq!(p.node.steps.len(), 1);
                assert!(matches!(p.node.steps[0], ast::PathStep::PathForEach));
            }
            other => panic!("expected Path with PathForEach step, got {:?}", other),
        }
    }

    #[test]
    fn test_precedence_mul_over_add() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let e = parse("1 + 2 * 3");
        if let ast::Expr::BinOp(node) = &e {
            assert_eq!(node.node.kind, ast::BinOpKind::Add);
            assert!(matches!(*node.node.rhs, ast::Expr::BinOp(_)));
        } else {
            panic!("Expected BinOp, got {:?}", e);
        }
    }

    #[test]
    fn test_precedence_and_over_or() {
        // a OR b AND c should parse as a OR (b AND c)
        let e = parse("a OR b AND c");
        if let ast::Expr::BinOp(node) = &e {
            assert_eq!(node.node.kind, ast::BinOpKind::Or);
        } else {
            panic!("Expected BinOp(Or), got {:?}", e);
        }
    }

    #[test]
    fn test_concat() {
        let e = parse("'a' || 'b'");
        if let ast::Expr::BinOp(node) = &e {
            assert_eq!(node.node.kind, ast::BinOpKind::Concat);
        } else {
            panic!("Expected Concat");
        }
    }

    #[test]
    fn test_complex_where() {
        // Real FDE query pattern
        let e = parse("u.email = 'test@co.com' AND p.originalPlatformId = 'GChat'");
        assert!(matches!(e, ast::Expr::BinOp(_)));
    }

    // ── Function call tests ──────────────

    #[test]
    fn test_call_no_args() {
        let e = parse("NOW()");
        assert!(matches!(&e, ast::Expr::Call(_)));
        if let ast::Expr::Call(n) = &e {
            assert_eq!(n.node.func_name.value, "NOW");
            assert!(n.node.args.is_empty());
        }
    }

    #[test]
    fn test_call_agg_star() {
        // COUNT is an aggregate — produces CallAgg, not Call
        let e = parse("COUNT(*)");
        assert!(matches!(&e, ast::Expr::CallAgg(_)));
        if let ast::Expr::CallAgg(n) = &e {
            assert_eq!(n.node.func_name.value, "COUNT");
            assert_eq!(n.node.args.len(), 1);
            assert!(matches!(n.node.args[0].node, CallArg::Star()));
        }
    }

    #[test]
    fn test_call_single_arg() {
        let e = parse("UPPER('hello')");
        assert!(matches!(&e, ast::Expr::Call(_)));
        if let ast::Expr::Call(n) = &e {
            assert_eq!(n.node.func_name.value, "UPPER");
            assert_eq!(n.node.args.len(), 1);
        }
    }

    #[test]
    fn test_call_multiple_args() {
        let e = parse("SUBSTRING(name, 1, 3)");
        assert!(matches!(&e, ast::Expr::Call(_)));
        if let ast::Expr::Call(n) = &e {
            assert_eq!(n.node.func_name.value, "SUBSTRING");
            assert_eq!(n.node.args.len(), 3);
        }
    }

    #[test]
    fn test_call_nested() {
        let e = parse("UPPER(TRIM(name))");
        assert!(matches!(&e, ast::Expr::Call(_)));
        if let ast::Expr::Call(n) = &e {
            assert_eq!(n.node.func_name.value, "UPPER");
            assert_eq!(n.node.args.len(), 1);
        }
    }
}
