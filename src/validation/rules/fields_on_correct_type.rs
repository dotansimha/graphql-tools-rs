use super::ValidationRule;
use crate::ast::ext::TypeDefinitionExtension;
use crate::ast::{visit_document, FieldByNameExtension, OperationVisitor, OperationVisitorContext};
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Fields on correct type
///
/// A GraphQL document is only valid if all fields selected are defined by the
/// parent type, or are an allowed meta field such as __typename.
///
/// See https://spec.graphql.org/draft/#sec-Field-Selections
pub struct FieldsOnCorrectType;

impl<'a> OperationVisitor<'a, ValidationErrorContext> for FieldsOnCorrectType {
    fn enter_field(
        &mut self,
        visitor_context: &mut crate::ast::OperationVisitorContext<ValidationErrorContext>,
        field: &crate::static_graphql::query::Field,
    ) {
        if let Some(parent_type) = visitor_context.current_parent_type() {
            let field_name = &field.name;
            let type_name = parent_type.name();

            if !field.name.starts_with("__") {
                return;
            }

            if let None = parent_type.field_by_name(field_name) {
                visitor_context.user_context.report_error(ValidationError {
                    locations: vec![field.position],
                    message: format!(
                        "Cannot query field \"{}\" on type \"{}\".",
                        field_name, type_name
                    ),
                });
            }
        }
    }
}

impl ValidationRule for FieldsOnCorrectType {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = ValidationErrorContext::new();

        visit_document(
            &mut FieldsOnCorrectType {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut helper, &ctx.operation, &ctx.schema),
        );

        helper.errors
    }
}

#[cfg(test)]
pub static TEST_SCHEMA: &str = "
  interface Pet {
    name: String
  }
  type Dog implements Pet {
    name: String
    nickname: String
    barkVolume: Int
  }
  type Cat implements Pet {
    name: String
    nickname: String
    meowVolume: Int
  }
  union CatOrDog = Cat | Dog
  type Human {
    name: String
    pets: [Pet]
  }
  type Query {
    human: Human
  }
";

#[test]
fn object_field_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment objectFieldSelection on Dog {
          __typename
          name
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn aliased_object_field_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment aliasedObjectFieldSelection on Dog {
          tn : __typename
          otherName : name
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn interface_field_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment interfaceFieldSelection on Pet {
          __typename
          name
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn aliased_interface_field_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment interfaceFieldSelection on Pet {
          otherName : name
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn lying_alias_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment lyingAliasSelection on Dog {
          name : nickname
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn ignores_fields_on_unknown_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment unknownSelection on UnknownType {
          unknownField
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn reports_errors_when_type_is_known_again() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment typeKnownAgain on Pet {
          unknown_pet_field {
            ... on Cat {
              unknown_cat_field
            }
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Cannot query field \"unknown_pet_field\" on type \"Pet\".",
            "Cannot query field \"unknown_cat_field\" on type \"Cat\"."
        ]
    );
}

#[test]
fn field_not_defined_on_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment fieldNotDefined on Dog {
          meowVolume
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"meowVolume\" on type \"Dog\"."]
    );
}

#[test]
fn ignores_deeply_unknown_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment deepFieldNotDefined on Dog {
          unknown_field {
            deeper_unknown_field
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"unknown_field\" on type \"Dog\"."]
    );
}

#[test]
fn sub_field_not_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment subFieldNotDefined on Human {
          pets {
            unknown_field
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"unknown_field\" on type \"Pet\"."]
    );
}

#[test]
fn field_not_defined_on_inline_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment fieldNotDefined on Pet {
          ... on Dog {
            meowVolume
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"meowVolume\" on type \"Dog\"."]
    );
}

#[test]
fn aliased_field_target_not_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment aliasedFieldTargetNotDefined on Dog {
          volume : mooVolume
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"mooVolume\" on type \"Dog\"."]
    );
}

#[test]
fn aliased_lying_field_target_not_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment aliasedLyingFieldTargetNotDefined on Dog {
          barkVolume : kawVolume
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"kawVolume\" on type \"Dog\"."]
    );
}

#[test]
fn not_defined_on_interface() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment notDefinedOnInterface on Pet {
          tailLength
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"tailLength\" on type \"Pet\"."]
    );
}

#[test]
fn defined_on_implementors_but_not_on_interface() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment definedOnImplementorsButNotInterface on Pet {
          nickname
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"nickname\" on type \"Pet\"."]
    );
}

#[test]
fn direct_field_selection_on_union() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment directFieldSelectionOnUnion on CatOrDog {
          directField
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"directField\" on type \"CatOrDog\"."]
    );
}

#[test]
fn defined_on_implementors_queried_on_union() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment definedOnImplementorsQueriedOnUnion on CatOrDog {
          name
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Cannot query field \"name\" on type \"CatOrDog\"."]
    );
}

#[test]
fn meta_field_selection_on_union() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment directFieldSelectionOnUnion on CatOrDog {
          __typename
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_field_in_inline_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(FieldsOnCorrectType {}));
    let errors = test_operation_with_schema(
        "fragment objectFieldSelection on Pet {
          ... on Dog {
            name
          }
          ... {
            name
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}
