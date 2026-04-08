//! DML statement parsing — INSERT, DELETE, REPLACE, UPSERT.
//!
//! Each DML type is a separate strategy implementing `DmlStrategy`.
//! `DmlParser` chains them with backtracking, same pattern as
//! `ComparisonStrategy` and `FromClauseParser`.

pub mod delete_strategy;
pub mod insert_strategy;
pub mod replace_strategy;
pub mod update_strategy;
pub mod upsert_strategy;

use partiql_ast::ast;
use winnow::prelude::*;

use crate::expr::ExprChain;
use crate::parse_context::ParseContext;

/// Each DML type implements this trait.
/// Holds `&ExprChain` in the struct for expression delegation.
pub trait DmlStrategy: Send + Sync {
    fn parse(&self, input: &mut &str, pctx: &ParseContext) -> PResult<ast::Dml>;
}

/// DML parser — chains DmlStrategy implementations.
pub struct DmlParser<'p> {
    strategies: Vec<Box<dyn DmlStrategy + 'p>>,
}

impl<'p> DmlParser<'p> {
    pub fn new(chain: &'p ExprChain) -> Self {
        Self {
            strategies: vec![
                Box::new(insert_strategy::InsertStrategy::new(chain)),
                Box::new(replace_strategy::ReplaceStrategy::new(chain)),
                Box::new(upsert_strategy::UpsertStrategy::new(chain)),
                Box::new(update_strategy::UpdateStrategy::new(chain)),
                Box::new(delete_strategy::DeleteStrategy::new(chain)),
            ],
        }
    }

    /// Try each DML strategy in order. Returns the first match.
    pub fn try_parse(&self, input: &mut &str, pctx: &ParseContext) -> Option<PResult<ast::Dml>> {
        for strategy in &self.strategies {
            let checkpoint = *input;
            match strategy.parse(input, pctx) {
                Ok(dml) => return Some(Ok(dml)),
                Err(winnow::error::ErrMode::Backtrack(_)) => {
                    *input = checkpoint;
                }
                Err(e) => return Some(Err(e)),
            }
        }
        None
    }
}
