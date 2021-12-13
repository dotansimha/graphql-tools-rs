use super::{DefaultVisitorContext, QueryVisitor, SchemaVisitor};
use crate::static_graphql::*;

pub enum ASTKind {
  Schema(schema::Document),
  Operation(query::Document)
}

pub trait ASTVisitor<T = DefaultVisitorContext>: QueryVisitor<T> + SchemaVisitor<T> {
  fn visit_ast(&mut self, ast: ASTKind, visitor_context: &mut T) {
    match ast {
      ASTKind::Schema(schema) => {
        self.visit_schema_document(&schema, visitor_context)
      }
      ASTKind::Operation(operation) => {
        self.visit_document(&operation, visitor_context)
      }
    }
  }
}
