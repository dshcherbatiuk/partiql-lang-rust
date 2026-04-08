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
    Call, CallArg, CaseSensitivity, ScopeQualifier, SymbolPrimitive, VarRef,
};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::identifier;
use crate::keyword::ch;
use crate::literal::boolean_strategy::BooleanLiteralStrategy;
use crate::literal::null_strategy::NullMissingStrategy;
use crate::literal::number_strategy::NumericLiteralStrategy;
use crate::literal::string_strategy::StringLiteralStrategy;
use crate::literal::LiteralStrategy;
use crate::whitespace::ws0;

pub struct PrimaryStrategy {
    literal_strategies: Vec<Box<dyn LiteralStrategy>>,
}

impl PrimaryStrategy {
    pub fn new() -> Self {
        Self {
            literal_strategies: vec![
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

impl ExprStrategy for PrimaryStrategy {
    fn parse<'a>(&self, input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
        let _ = ws0(input);

        // Try each literal strategy
        for strategy in &self.literal_strategies {
            let checkpoint = *input;
            match strategy.parse(input, ctx) {
                Ok(expr) => return Ok(expr),
                Err(winnow::error::ErrMode::Backtrack(_)) => {
                    *input = checkpoint;
                }
                Err(e) => return Err(e),
            }
        }

        // Parenthesized expression: ( expr )
        if let Ok(expr) = parse_paren_expr(input, ctx) {
            return Ok(expr);
        }

        // Function call or identifier (last — catches anything that looks like a name)
        parse_identifier_or_call(input, ctx)
    }

    fn name(&self) -> &str {
        "Primary"
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
fn parse_identifier_or_call<'a>(
    input: &mut &'a str,
    ctx: &StrategyContext<'_>,
) -> PResult<ast::Expr> {
    let name = identifier::identifier(input)?;
    let _ = ws0(input);

    // Function call: name(args...)
    if ch('(').parse_next(input).is_ok() {
        let _ = ws0(input);

        let mut args = Vec::new();

        // Empty args: name()
        if ch(')').parse_next(input).is_ok() {
            return Ok(ast::Expr::Call(ctx.node(Call {
                func_name: SymbolPrimitive {
                    value: name,
                    case: CaseSensitivity::CaseInsensitive,
                },
                args,
            })));
        }

        // Star arg: COUNT(*)
        if ch('*').parse_next(input).is_ok() {
            args.push(ctx.node(CallArg::Star()));
            let _ = ws0(input);
            ch(')').parse_next(input)?;
            return Ok(ast::Expr::Call(ctx.node(Call {
                func_name: SymbolPrimitive {
                    value: name,
                    case: CaseSensitivity::CaseInsensitive,
                },
                args,
            })));
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

        return Ok(ast::Expr::Call(ctx.node(Call {
            func_name: SymbolPrimitive {
                value: name,
                case: CaseSensitivity::CaseInsensitive,
            },
            args,
        })));
    }

    Ok(ast::Expr::VarRef(ctx.node(VarRef {
        name: SymbolPrimitive {
            value: name,
            case: CaseSensitivity::CaseInsensitive,
        },
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
    fn test_ion_string() {
        assert!(matches!(
            &parse(r#""hello""#),
            ast::Expr::Lit(n) if matches!(&n.node, Lit::CharStringLit(s) if s == "hello")
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
    fn test_call_star() {
        let e = parse("COUNT(*)");
        assert!(matches!(&e, ast::Expr::Call(_)));
        if let ast::Expr::Call(n) = &e {
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
