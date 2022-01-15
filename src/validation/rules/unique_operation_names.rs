use std::collections::HashMap;

use super::ValidationRule;
use crate::ast::{visit_document, AstNodeWithName, OperationVisitor, OperationVisitorContext};
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::ValidationError;

/// Unique operation names
///
/// A GraphQL document is only valid if all defined operations have unique names.
///
/// See https://spec.graphql.org/draft/#sec-Operation-Name-Uniqueness
pub struct UniqueOperationNames;

impl<'a> OperationVisitor<'a, FoundOperations> for UniqueOperationNames {
    fn enter_operation_definition(
        &mut self,
        visitor_context: &mut OperationVisitorContext<FoundOperations>,
        operation_definition: &OperationDefinition,
    ) {
        if let Some(name) = operation_definition.node_name() {
            visitor_context.user_context.store_finding(&name);
        }
    }
}
struct FoundOperations {
    findings_counter: HashMap<String, i32>,
}

impl FoundOperations {
    fn new() -> Self {
        Self {
            findings_counter: HashMap::new(),
        }
    }

    fn store_finding(&mut self, name: &String) {
        let value = *self.findings_counter.entry(name.clone()).or_insert(0);
        self.findings_counter.insert(name.clone(), value + 1);
    }
}

impl ValidationRule for UniqueOperationNames {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut visitor_helper = FoundOperations::new();

        visit_document(
            &mut UniqueOperationNames {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut visitor_helper, &ctx.schema),
        );

        visitor_helper
            .findings_counter
            .into_iter()
            .filter(|(_key, value)| *value > 1)
            .map(|(key, _value)| ValidationError {
                message: format!("There can be only one operation named \"{}\".", key),
                locations: vec![],
            })
            .collect()
    }
}

#[test]
fn no_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Type {
          field 
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn one_anon_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "{
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn one_named_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "query Foo {
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "query Foo {
          field
        }
        query Bar {
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_operations_of_different_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "query Foo {
          field
        }
        mutation Bar {
          field
        }
        subscription Baz {
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn fragment_and_operation_named_the_same() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "query Foo {
          ...Foo
        }
        fragment Foo on Type {
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_operations_of_same_name() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "query Foo {
          fieldA
        }
        query Foo {
          fieldB
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one operation named \"Foo\".",]
    );
}

#[test]
fn multiple_ops_of_same_name_of_different_types_mutation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "query Foo {
          fieldA
        }
        mutation Foo {
          fieldB
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one operation named \"Foo\".",]
    );
}

#[test]
fn multiple_ops_of_same_name_of_different_types_subscription() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_with_schema(
        "query Foo {
          fieldA
        }
        subscription Foo {
          fieldB
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one operation named \"Foo\".",]
    );
}
