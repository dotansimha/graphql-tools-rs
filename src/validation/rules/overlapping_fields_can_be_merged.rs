use super::ValidationRule;
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

pub struct OverlappingFieldsCanBeMerged<'a> {
    ctx: ValidationContext<'a>,
}

struct FindConflicts {
    conflicts: &'static str,
}

impl<'a> QueryVisitor<'a> for OverlappingFieldsCanBeMerged<'a> {
    fn enter_selection_set(&mut self, _node: &'a graphql_parser::query::SelectionSet<String>) {}
}

impl<'a> ValidationRule<'a> for OverlappingFieldsCanBeMerged<'a> {
    fn validate(&mut self) {
        self.__visit_document(&ctx.operation)
    }
}
