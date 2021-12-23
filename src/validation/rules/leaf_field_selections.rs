use super::ValidationRule;
// use crate::static_graphql::query::*;
use crate::{
    ast::TypeInfoQueryVisitor,
    validation::utils::{ValidationContext, ValidationError},
};

/// Leaf Field Selections
///
/// Field selections on scalars or enums are never allowed, because they are the leaf nodes of any GraphQL operation.
///
/// https://spec.graphql.org/draft/#sec-Leaf-Field-Selections
pub struct LeafFieldSelections;

impl TypeInfoQueryVisitor<ValidationContext<'_>> for LeafFieldSelections {}

impl ValidationRule for LeafFieldSelections {
    fn validate<'a>(&self, _ctx: &ValidationContext) -> Vec<ValidationError> {
        return vec![];
        // self.visit_document(
        //     &ctx.operation.clone(),
        //     ctx,
        //     ctx.type_info
        //         .expect("Rule LeafFieldSelections requires type-info to be available"),
        // );
    }
}
