use std::collections::hash_map::Entry;
use std::collections::HashMap;

use graphql_parser::Pos;

use super::ValidationRule;
use crate::ast::{visit_document, OperationVisitor, OperationVisitorContext};
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Unique variable names
///
/// A GraphQL operation is only valid if all its variables are uniquely named.
///
/// See https://spec.graphql.org/draft/#sec-Variable-Uniqueness
pub struct UniqueVariableNames {}

struct UniqueVariableNamesHelper {
    found_records: HashMap<String, Pos>,
    error_context: ValidationErrorContext,
}

impl UniqueVariableNamesHelper {
    fn new() -> Self {
        UniqueVariableNamesHelper {
            found_records: HashMap::new(),
            error_context: ValidationErrorContext::new(),
        }
    }
}

impl<'a> OperationVisitor<'a, UniqueVariableNamesHelper> for UniqueVariableNames {
    fn enter_operation_definition(
        &mut self,
        visitor_context: &mut OperationVisitorContext<UniqueVariableNamesHelper>,
        _operation_definition: &OperationDefinition,
    ) {
        visitor_context.user_context.found_records.clear();
    }

    fn enter_variable_definition(
        &mut self,
        visitor_context: &mut OperationVisitorContext<UniqueVariableNamesHelper>,
        variable_definition: &VariableDefinition,
    ) {
        match visitor_context
            .user_context
            .found_records
            .entry(variable_definition.name.clone())
        {
            Entry::Occupied(entry) => {
                visitor_context
                    .user_context
                    .error_context
                    .report_error(ValidationError {
                        locations: vec![*entry.get(), variable_definition.position],
                        message: format!(
                            "There can only be one variable named \"${}\".",
                            variable_definition.name
                        ),
                    })
            }
            Entry::Vacant(entry) => {
                entry.insert(variable_definition.position);
            }
        };
    }
}

impl ValidationRule for UniqueVariableNames {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut visitor_helper = UniqueVariableNamesHelper::new();

        visit_document(
            &mut UniqueVariableNames {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut visitor_helper, &ctx.schema),
        );

        visitor_helper.error_context.errors
    }
}

#[test]
fn unique_variable_names() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueVariableNames {}));
    let errors = test_operation_with_schema(
        "query A($x: Int, $y: String) { __typename }
        query B($x: String, $y: Int) { __typename }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn duplicate_variable_names() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueVariableNames {}));
    let errors = test_operation_with_schema(
        "query A($x: Int, $x: Int, $x: String) { __typename }
        query B($y: String, $y: Int) { __typename }
        query C($z: Int, $z: Int) { __typename }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 4);
    assert!(messages.contains(&&"There can only be one variable named \"$x\".".to_owned()));
    assert!(messages.contains(&&"There can only be one variable named \"$y\".".to_owned()));
    assert!(messages.contains(&&"There can only be one variable named \"$z\".".to_owned()));
}
