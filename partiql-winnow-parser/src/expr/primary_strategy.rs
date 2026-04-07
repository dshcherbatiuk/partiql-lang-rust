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
use partiql_ast::ast::{CaseSensitivity, ScopeQualifier, SymbolPrimitive, VarRef};
use winnow::prelude::*;

use super::{ExprStrategy, StrategyContext};
use crate::identifier;
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

        // Identifier / variable reference (last — catches anything that looks like a name)
        parse_identifier(input, ctx)
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

fn parse_identifier<'a>(input: &mut &'a str, ctx: &StrategyContext<'_>) -> PResult<ast::Expr> {
    let name = identifier::identifier(input)?;
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
    use partiql_ast::ast::Lit;

    use crate::expr::ExprChain;

    fn parse(input: &str) -> ast::Expr {
        let chain = ExprChain::new();
        let mut i = input;
        chain.parse_expr(&mut i).expect("parse failed")
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
}
