use crate::ast::{QueryVisitor};
use super::ValidationRule;

pub struct OverlappingFieldsCanBeMerged {}

struct OverlappingFieldsCanBeMergedHelper {
  
}

impl<'a> QueryVisitor<'a> for OverlappingFieldsCanBeMerged {
  fn enter_selection_set(&mut self, _node: &'a graphql_parser::query::SelectionSet<String>) {
      
  }
}

impl<'a> ValidationRule<'a> for OverlappingFieldsCanBeMerged {
   fn validate(&mut self, ctx: &crate::validation::ValidationContext<'a>) {
      self.__visit_document(&ctx.operation)
   }
}