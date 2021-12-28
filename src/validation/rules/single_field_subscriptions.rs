use super::ValidationRule;
use crate::ast::{collect_fields, TypeInfo, TypeInfoQueryVisitor};
use crate::static_graphql::schema::TypeDefinition;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Unique operation names
///
/// A GraphQL document is only valid if all defined operations have unique names.
///
/// See https://spec.graphql.org/draft/#sec-Operation-Name-Uniqueness
pub struct SingleFieldSubscriptions;

impl<'a> TypeInfoQueryVisitor<ValidationErrorContext<'a>> for SingleFieldSubscriptions {
    fn enter_subscription(
        &self,
        _node: &crate::static_graphql::query::Subscription,
        _visitor_context: &mut ValidationErrorContext<'a>,
        _type_info: &TypeInfo,
    ) {
        if let Some(subscription_type) = _visitor_context
            .ctx
            .type_info_registry
            .as_ref()
            .unwrap()
            .subscription_type
        {
            let operation_name = _node.name.as_ref();

            let selection_set_fields = collect_fields(
                &_node.selection_set,
                &TypeDefinition::Object(subscription_type.clone()),
                &_visitor_context.ctx.fragments,
                _visitor_context.ctx.type_info_registry.as_ref().unwrap(),
            );

            if selection_set_fields.len() > 1 {
                let error_message = match operation_name {
                    Some(operation_name) => format!(
                        "Subscription \"{}\" must select only one top level field.",
                        operation_name
                    ),
                    None => {
                        "Anonymous Subscription must select only one top level field.".to_owned()
                    }
                };

                _visitor_context.errors.push(ValidationError {
                    locations: vec![_node.position],
                    message: error_message,
                });
            }

            selection_set_fields
                .into_iter()
                .filter_map(|(field_name, fields_records)| {
                    if field_name.starts_with("__") {
                        return Some((field_name, fields_records));
                    }

                    None
                })
                .for_each(|(_field_name, _fields_records)| {
                    let error_message = match operation_name {
                        Some(operation_name) => format!(
                            "Subscription \"{}\" must not select an introspection top level field.",
                            operation_name
                        ),
                        None => "Anonymous Subscription must not select an introspection top level field."
                            .to_owned(),
                    };

                    _visitor_context.errors.push(ValidationError {
                        locations: vec![_node.position],
                        message: error_message,
                    });
                })
        }
    }
}

impl ValidationRule for SingleFieldSubscriptions {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut error_context = ValidationErrorContext::new(ctx);

        if let Some(type_info_registry) = &ctx.type_info_registry {
            self.visit_document(
                &ctx.operation.clone(),
                &mut error_context,
                &type_info_registry,
            );
        }
        error_context.errors
    }
}

#[cfg(test)]
pub static TEST_SCHEMA_SUBSCRIPTION: &str = "
type Message {
  body: String
  sender: String
}
type SubscriptionRoot {
  importantEmails: [String]
  notImportantEmails: [String]
  moreImportantEmails: [String]
  spamEmails: [String]
  deletedEmails: [String]
  newMessage: Message
}
type QueryRoot {
  dummy: String
}
schema {
  query: QueryRoot
  subscription: SubscriptionRoot
}
";

#[test]
fn valid_subscription_with_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription sub {
          ...newMessageFields
        }
        fragment newMessageFields on SubscriptionRoot {
          newMessage {
            body
            sender
          }
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn valid_subscription_with_fragment_and_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription sub {
          newMessage {
            body
          }
          ...newMessageFields
        }
        fragment newMessageFields on SubscriptionRoot {
          newMessage {
            body
            sender
          }
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn fails_with_more_than_one_root_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription ImportantEmails {
          importantEmails
          notImportantEmails
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Subscription \"ImportantEmails\" must select only one top level field."]
    );
}

#[test]
fn fails_with_more_than_one_root_field_including_introspection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription ImportantEmails {
          importantEmails
          __typename
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Subscription \"ImportantEmails\" must select only one top level field.",
            "Subscription \"ImportantEmails\" must not select an introspection top level field."
        ]
    );
}

#[test]
fn fails_with_more_than_one_root_field_including_aliased_introspection_via_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription ImportantEmails {
          importantEmails
          ...Introspection
        }
        fragment Introspection on SubscriptionRoot {
          typename: __typename
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Subscription \"ImportantEmails\" must select only one top level field.",
            "Subscription \"ImportantEmails\" must not select an introspection top level field."
        ]
    );
}

#[test]
fn fails_with_many_more_than_one_root_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription ImportantEmails {
          importantEmails
          notImportantEmails
          spamEmails
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Subscription \"ImportantEmails\" must select only one top level field.",]
    );
}

#[test]
fn fails_with_many_more_than_one_root_field_via_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription ImportantEmails {
          importantEmails
          ... {
            more: moreImportantEmails
          }
          ...NotImportantEmails
        }
        fragment NotImportantEmails on SubscriptionRoot {
          notImportantEmails
          deleted: deletedEmails
          ...SpamEmails
        }
        fragment SpamEmails on SubscriptionRoot {
          spamEmails
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Subscription \"ImportantEmails\" must select only one top level field.",]
    );
}

#[test]
fn does_not_infinite_loop_on_recursive_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription NoInfiniteLoop {
          ...A
        }
        fragment A on SubscriptionRoot {
          ...A
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn fails_with_many_more_than_one_root_field_via_fragments_anonymous() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription {
          importantEmails
          ... {
            more: moreImportantEmails
            ...NotImportantEmails
          }
          ...NotImportantEmails
        }
        fragment NotImportantEmails on SubscriptionRoot {
          notImportantEmails
          deleted: deletedEmails
          ... {
            ... {
              archivedEmails
            }
          }
          ...SpamEmails
        }
        fragment SpamEmails on SubscriptionRoot {
          spamEmails
          ...NonExistentFragment
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Anonymous Subscription must select only one top level field.",]
    );
}

#[test]
fn fails_with_more_than_one_root_field_in_anonymous_subscriptions() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription {
          importantEmails
          notImportantEmails
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Anonymous Subscription must select only one top level field.",]
    );
}

#[test]
fn fails_with_introspection_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription ImportantEmails {
          __typename
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Subscription \"ImportantEmails\" must not select an introspection top level field."]
    );
}

#[test]
fn fails_with_introspection_field_in_anonymous_subscription() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription { 
          __typename
        }",
        TEST_SCHEMA_SUBSCRIPTION,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Anonymous Subscription must not select an introspection top level field."]
    );
}

#[test]
fn skips_if_not_subscription_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(SingleFieldSubscriptions {}));
    let errors = test_operation_with_schema(
        "subscription {
          __typename
        }",
        "type Query {
          dummy: String
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}
