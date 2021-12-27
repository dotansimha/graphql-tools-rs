use std::collections::HashMap;

use super::ValidationRule;
use crate::ast::AstNodeWithName;
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationError;
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// Unique operation names
///
/// A GraphQL document is only valid if all defined operations have unique names.
///
/// See https://spec.graphql.org/draft/#sec-Operation-Name-Uniqueness
pub struct UniqueOperationNames;

impl QueryVisitor<FoundOperations> for UniqueOperationNames {
    fn enter_operation_definition(
        &self,
        node: &OperationDefinition,
        visitor_context: &mut FoundOperations,
    ) {
        if let Some(name) = node.node_name() {
            visitor_context.store_finding(&name);
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
        let mut found_operations = FoundOperations::new();
        self.visit_document(&ctx.operation.clone(), &mut found_operations);

        found_operations
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
    let errors = test_operation_without_schema(
        "fragment fragA on Type {
          field
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn one_anon_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_without_schema(
        "{
          field
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn one_named_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_without_schema(
        "query Foo {
          field
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_without_schema(
        "query Foo {
          field
        }
        query Bar {
          field
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_operations_of_different_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_without_schema(
        "query Foo {
          field
        }
        mutation Bar {
          field
        }
        subscription Baz {
          field
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn fragment_and_operation_named_the_same() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_without_schema(
        "query Foo {
          ...Foo
        }
        fragment Foo on Type {
          field
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_operations_of_same_name() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueOperationNames {}));
    let errors = test_operation_without_schema(
        "query Foo {
          fieldA
        }
        query Foo {
          fieldB
        }",
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
    let errors = test_operation_without_schema(
        "query Foo {
          fieldA
        }
        mutation Foo {
          fieldB
        }",
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
    let errors = test_operation_without_schema(
        "query Foo {
          fieldA
        }
        subscription Foo {
          fieldB
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one operation named \"Foo\".",]
    );
}
