use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::static_graphql::schema::TypeDefinition;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// Fragments on composite type
///
/// Fragments use a type condition to determine if they apply, since fragments
/// can only be spread into a composite type (object, interface, or union), the
/// type condition must also be a composite type.
///
/// https://spec.graphql.org/draft/#sec-Fragments-On-Composite-Types
pub struct FragmentsOnCompositeTypes;

impl QueryVisitor<ValidationErrorContext<'_>> for FragmentsOnCompositeTypes {
    fn enter_inline_fragment(
        &self,
        _node: &InlineFragment,
        _visitor_context: &mut ValidationErrorContext,
    ) {
        if let Some(TypeCondition::On(type_condition)) = &_node.type_condition {
            let gql_type = _visitor_context
                .ctx
                .find_schema_definition_by_name(type_condition.to_owned());

            if let Some(gql_type) = gql_type {
                match gql_type {
                    TypeDefinition::Object(_)
                    | TypeDefinition::Interface(_)
                    | TypeDefinition::Union(_) => {}
                    _ => _visitor_context.report_error(ValidationError {
                        locations: vec![_node.position],
                        message: format!(
                            "Fragment cannot condition on non composite type \"{}\".",
                            type_condition
                        ),
                    }),
                }
            }
        }
    }

    fn enter_fragment_definition(
        &self,
        _node: &FragmentDefinition,
        _visitor_context: &mut ValidationErrorContext,
    ) {
        let TypeCondition::On(type_condition) = &_node.type_condition;

        if let Some(gql_type) = _visitor_context
            .ctx
            .find_schema_definition_by_name(type_condition.to_owned())
        {
            match gql_type {
                TypeDefinition::Object(_)
                | TypeDefinition::Interface(_)
                | TypeDefinition::Union(_) => {}
                _ => _visitor_context.report_error(ValidationError {
                    locations: vec![_node.position],
                    message: format!(
                        "Fragment \"{}\" cannot condition on non composite type \"{}\".",
                        _node.name, type_condition
                    ),
                }),
            }
        }
    }
}

impl ValidationRule for FragmentsOnCompositeTypes {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut error_context = ValidationErrorContext::new(ctx);
        self.visit_document(&ctx.operation.clone(), &mut error_context);

        error_context.errors
    }
}

#[test]
fn object_is_valid_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_without_schema(
        "fragment validFragment on Dog {
          barks
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn interface_is_valid_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_without_schema(
        "fragment validFragment on Pet {
          name
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn object_is_valid_inline_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_without_schema(
        "fragment validFragment on Pet {
          ... on Dog {
            barks
          }
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn interface_is_valid_inline_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_without_schema(
        "fragment validFragment on Mammal {
          ... on Canine {
            name
          }
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn inline_fragment_without_type_is_valid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_without_schema(
        "fragment validFragment on Pet {
          ... {
            name
          }
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn union_is_valid_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_without_schema(
        "fragment validFragment on CatOrDog {
          __typename
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn scalar_is_invalid_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_with_schema(
        "fragment scalarFragment on Boolean {
          bad
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Fragment \"scalarFragment\" cannot condition on non composite type \"Boolean\"."]
    );
}

#[test]
fn enum_is_invalid_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_with_schema(
        "fragment scalarFragment on FurColor {
          bad
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Fragment \"scalarFragment\" cannot condition on non composite type \"FurColor\"."]
    );
}

#[test]
fn input_object_is_invalid_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_with_schema(
        "fragment inputFragment on ComplexInput {
          stringField
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Fragment \"inputFragment\" cannot condition on non composite type \"ComplexInput\"."]
    );
}

#[test]
fn scalar_is_invalid_inline_fragment_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FragmentsOnCompositeTypes {}));
    let errors = test_operation_with_schema(
        "fragment invalidFragment on Pet {
          ... on String {
            barks
          }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Fragment cannot condition on non composite type \"String\"."]
    );
}
