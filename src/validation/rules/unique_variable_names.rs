use std::collections::HashMap;

use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{
    ast::{ext::AstWithVariables, QueryVisitor},
    validation::utils::ValidationContext,
};

/// Unique variable names
///
/// A GraphQL operation is only valid if all its variables are uniquely named.
///
/// See https://spec.graphql.org/draft/#sec-Variable-Uniqueness
pub struct UniqueVariableNames;

struct UniqueVariableNamesHelper<'a> {
    error_context: ValidationErrorContext<'a>,
}

impl<'a> UniqueVariableNamesHelper<'a> {
    fn new(validation_context: &'a ValidationContext<'a>) -> Self {
        UniqueVariableNamesHelper {
            error_context: ValidationErrorContext::new(validation_context),
        }
    }
}

impl<'a> QueryVisitor<UniqueVariableNamesHelper<'a>> for UniqueVariableNames {
    fn leave_operation_definition(
        &self,
        node: &OperationDefinition,
        visitor_context: &mut UniqueVariableNamesHelper<'a>,
    ) {
        let variables = node.get_variables();

        let mut seen_variables = HashMap::new();

        variables.iter().for_each(|var| {
            if seen_variables.contains_key(&var.name) {
                visitor_context.error_context.report_error(ValidationError {
                    locations: vec![],
                    message: format!("There can only be one variable named \"${}\".", var.name),
                });
            } else {
                seen_variables.insert(var.name.clone(), true);
            }
        })
    }
}

impl ValidationRule for UniqueVariableNames {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = UniqueVariableNamesHelper::new(&ctx);
        self.visit_document(&ctx.operation.clone(), &mut helper);

        helper.error_context.errors
    }
}

#[test]
fn unique_variable_names() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueVariableNames {}));
    let errors = test_operation_without_schema(
        "query A($x: Int, $y: String) { __typename }
        query B($x: String, $y: Int) { __typename }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn duplicate_variable_names() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueVariableNames {}));
    let errors = test_operation_without_schema(
        "query A($x: Int, $x: Int, $x: String) { __typename }
        query B($y: String, $y: Int) { __typename }
        query C($z: Int, $z: Int) { __typename }",
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 4);
    assert!(messages.contains(&&"There can only be one variable named \"$x\".".to_owned()));
    assert!(messages.contains(&&"There can only be one variable named \"$y\".".to_owned()));
    assert!(messages.contains(&&"There can only be one variable named \"$z\".".to_owned()));
}
