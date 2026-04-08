#![deny(rust_2018_idioms)]
#![deny(clippy::all)]

use crate::lower::AstToLogical;

use partiql_ast::ast;
use partiql_ast_passes::error::AstTransformationError;
use partiql_ast_passes::name_resolver::NameResolver;
use partiql_logical as logical;

use partiql_catalog::catalog::SharedCatalog;

mod builtins;
mod datetime_functions;
mod functions;
mod graph;
mod lower;
mod typer;

pub struct LogicalPlanner<'c> {
    catalog: &'c dyn SharedCatalog,
}

impl<'c> LogicalPlanner<'c> {
    pub fn new(catalog: &'c dyn SharedCatalog) -> Self {
        LogicalPlanner { catalog }
    }

    #[inline]
    pub fn lower(
        &self,
        ast: &ast::AstNode<ast::TopLevelQuery>,
    ) -> Result<logical::LogicalPlan<logical::BindingsOp>, AstTransformationError> {
        let mut resolver = NameResolver::new(self.catalog);
        let registry = resolver.resolve(ast)?;
        let planner = AstToLogical::new(self.catalog, registry);
        planner.lower_query(ast)
    }
}
