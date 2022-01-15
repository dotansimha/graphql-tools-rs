use std::collections::HashMap;

use super::ValidationRule;
use crate::ast::{visit_document, AstNodeWithName, OperationVisitor, OperationVisitorContext};
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::ValidationError;

/// Unique fragment names
///
/// A GraphQL document is only valid if all defined fragments have unique names.
///
/// See https://spec.graphql.org/draft/#sec-Fragment-Name-Uniqueness
pub struct UniqueFragmentNames;

impl<'a> OperationVisitor<'a, FoundFragments> for UniqueFragmentNames {
    fn enter_fragment_definition(
        &mut self,
        visitor_context: &mut crate::ast::OperationVisitorContext<FoundFragments>,
        fragment: &FragmentDefinition,
    ) {
        if let Some(name) = fragment.node_name() {
            visitor_context.user_context.store_finding(&name);
        }
    }
}

struct FoundFragments {
    findings_counter: HashMap<String, i32>,
}

impl FoundFragments {
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

impl ValidationRule for UniqueFragmentNames {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut visitor_helper = FoundFragments::new();

        visit_document(
            &mut UniqueFragmentNames {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut visitor_helper, &ctx.operation, &ctx.schema),
        );

        visitor_helper
            .findings_counter
            .into_iter()
            .filter(|(_key, value)| *value > 1)
            .map(|(key, _value)| ValidationError {
                message: format!("There can be only one fragment named \"{}\".", key),
                locations: vec![],
            })
            .collect()
    }
}

#[test]
fn no_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueFragmentNames {}));
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
