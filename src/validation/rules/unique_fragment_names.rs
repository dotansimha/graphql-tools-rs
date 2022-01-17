use std::collections::HashMap;

use super::ValidationRule;
use crate::ast::{visit_document, AstNodeWithName, OperationVisitor, OperationVisitorContext};
use crate::static_graphql::query::*;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Unique fragment names
///
/// A GraphQL document is only valid if all defined fragments have unique names.
///
/// See https://spec.graphql.org/draft/#sec-Fragment-Name-Uniqueness
pub struct UniqueFragmentNames {
    findings_counter: HashMap<String, i32>,
}

impl<'a> OperationVisitor<'a, ValidationErrorContext> for UniqueFragmentNames {
    fn enter_fragment_definition(
        &mut self,
        _: &mut OperationVisitorContext,
        _: &mut ValidationErrorContext,
        fragment: &FragmentDefinition,
    ) {
        if let Some(name) = fragment.node_name() {
            self.store_finding(&name);
        }
    }
}

impl UniqueFragmentNames {
  pub fn new() -> Self {
        Self {
            findings_counter: HashMap::new(),
        }
    }

    fn store_finding(&mut self, name: &String) {
        let value = *self.findings_counter.entry(name.clone()).or_insert(0);
        self.findings_counter.insert(name.clone(), value + 1);
    }
}

impl ValidationRule for UniqueFragmentNames {
    fn validate<'a>(
        &self,
        ctx: &'a mut OperationVisitorContext,
        error_collector: &mut ValidationErrorContext,
    ) {
        let mut rule = UniqueFragmentNames::new();

        visit_document(&mut rule, &ctx.operation, ctx, error_collector);

        rule.findings_counter
            .into_iter()
            .filter(|(_key, value)| *value > 1)
            .for_each(|(key, _value)| {
                error_collector.report_error(ValidationError {
                    message: format!("There can be only one fragment named \"{}\".", key),
                    locations: vec![],
                })
            })
    }
}

#[test]
fn no_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames::new()));
    let errors = test_operation_with_schema(
        "{
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn one_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames::new()));
    let errors = test_operation_with_schema(
        "{
          ...fragA
        }
        fragment fragA on Type {
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn many_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames::new()));
    let errors = test_operation_with_schema(
        "{
          ...fragA
          ...fragB
          ...fragC
        }
        fragment fragA on Type {
          fieldA
        }
        fragment fragB on Type {
          fieldB
        }
        fragment fragC on Type {
          fieldC
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn inline_fragments_are_always_unique() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames::new()));
    let errors = test_operation_with_schema(
        "{
          ...on Type {
            fieldA
          }
          ...on Type {
            fieldB
          }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn fragment_and_operation_named_the_same() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames::new()));
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

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn fragments_named_the_same() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames::new()));
    let errors = test_operation_with_schema(
        "{
          ...fragA
        }
        fragment fragA on Type {
          fieldA
        }
        fragment fragA on Type {
          fieldB
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one fragment named \"fragA\"."]
    );
}

#[test]
fn fragments_named_the_same_without_being_referenced() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames::new()));
    let errors = test_operation_with_schema(
        "fragment fragA on Type {
          fieldA
        }
        fragment fragA on Type {
          fieldB
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one fragment named \"fragA\"."]
    );
}
