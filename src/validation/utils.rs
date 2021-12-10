use std::collections::HashMap;

use crate::ast::{DefaultVisitorContext, QueryVisitor};
use thiserror::Error;

pub struct ValidationContext<'a> {
    pub operation: graphql_parser::query::Document<'a, String>,
    pub schema: graphql_parser::schema::Document<'a, String>,
    // pub fragments: HashMap<String, &'a graphql_parser::query::FragmentDefinition<'a, String>>,
}

impl<'a> ValidationContext<'a> {
    pub fn new(
        operation: &'a graphql_parser::query::Document<'a, String>,
        schema: &'a graphql_parser::schema::Document<'a, String>,
    ) -> Self {
        ValidationContext {
            operation: operation.clone(),
            schema: schema.clone(),
        }
    }
}

#[derive(Error, Debug)]
#[error("GraphQL Validation Error: {}", _0)]
pub struct ValidationError(String);

pub trait ValidationRuleQueryVisitor {}

pub struct LocateFragments<'a> {
    pub located_fragments:
        HashMap<String, &'a graphql_parser::query::FragmentDefinition<'a, String>>,
}

impl<'a> QueryVisitor<'a> for LocateFragments<'a> {
    fn enter_fragment_definition(
        &mut self,
        _node: &'a graphql_parser::query::FragmentDefinition<'a, String>,
        _ctx: &DefaultVisitorContext,
    ) {
        self.located_fragments.insert(_node.name.clone(), _node);
    }
}

impl<'a> LocateFragments<'a> {
    pub fn locate_fragments(&mut self, operation: &'a graphql_parser::query::Document<'a, String>) {
        self.visit_document(operation, &DefaultVisitorContext {});
    }
}
