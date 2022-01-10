use super::ValidationRule;
use crate::ast::ext::TypeDefinitionExtension;
use crate::ast::{TypeInfo, TypeInfoElementRef, TypeInfoQueryVisitor};
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Known argument names
///
/// A GraphQL field/directive is only valid if all supplied arguments are defined by
/// that field.
///
/// See https://spec.graphql.org/draft/#sec-Argument-Names
/// See https://spec.graphql.org/draft/#sec-Directives-Are-In-Valid-Locations
pub struct KnownArgumentNames;

impl<'a> TypeInfoQueryVisitor<ValidationErrorContext<'a>> for KnownArgumentNames {
    fn enter_directive(
        &self,
        directive: &crate::static_graphql::query::Directive,
        visitor_context: &mut ValidationErrorContext<'a>,
        _type_info: &TypeInfo,
    ) {
        let known_directives = &visitor_context
            .ctx
            .type_info_registry
            .as_ref()
            .unwrap()
            .directives;

        if let Some(directive_def) = known_directives.get(&directive.name) {
            let known_directive_args = &directive_def.arguments;

            for (directive_arg_name, _directive_value) in &directive.arguments {
                if let None = known_directive_args
                    .iter()
                    .find(|input_value| input_value.name.eq(directive_arg_name))
                {
                    visitor_context.report_error(ValidationError {
                        message: format!(
                            "Unknown argument \"{}\" on directive \"@{}\".",
                            directive_arg_name, directive.name
                        ),
                        locations: vec![],
                    })
                }
            }
        }
    }

    fn enter_field_argument(
        &self,
        argument_name: &String,
        _value: &crate::static_graphql::query::Value,
        _parent_field: &crate::static_graphql::query::Field,
        _visitor_context: &mut ValidationErrorContext<'a>,
        type_info: &TypeInfo,
    ) {
        if let Some(TypeInfoElementRef::Empty) = type_info.get_argument() {
            if let Some(TypeInfoElementRef::Ref(field_def)) = type_info.get_field_def() {
                if let Some(TypeInfoElementRef::Ref(parent_type)) = type_info.get_parent_type() {
                    _visitor_context.report_error(ValidationError {
                        locations: vec![_parent_field.position],
                        message: format!(
                            "Unknown argument \"{}\" on field \"{}.{}\".",
                            argument_name,
                            parent_type.name(),
                            field_def.name
                        ),
                    });
                }
            }
        }
    }
}

impl ValidationRule for KnownArgumentNames {
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

#[test]
fn single_arg_is_known() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment argOnRequiredArg on Dog {
          doesKnowCommand(dogCommand: SIT)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn multple_args_are_known() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment multipleArgs on ComplicatedArgs {
          multipleReqs(req1: 1, req2: 2)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn ignores_args_of_unknown_fields() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment argOnUnknownField on Dog {
          unknownField(unknownArg: SIT)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn multiple_args_in_reverse_order_are_known() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment multipleArgsReverseOrder on ComplicatedArgs {
          multipleReqs(req2: 2, req1: 1)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn no_args_on_optional_arg() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment noArgOnOptionalArg on Dog {
          isHouseTrained
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn args_are_known_deeply() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "{
          dog {
            doesKnowCommand(dogCommand: SIT)
          }
          human {
            pet {
              ... on Dog {
                doesKnowCommand(dogCommand: SIT)
              }
            }
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn directive_args_are_known() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "{
          dog @skip(if: true)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn field_args_are_invalid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "{
          dog @skip(unless: true)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Unknown argument \"unless\" on directive \"@skip\"."]
    );
}

#[test]
fn directive_without_args_is_valid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        " {
          dog @onField
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn arg_passed_to_directive_without_arg_is_reported() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        " {
          dog @onField(if: true)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Unknown argument \"if\" on directive \"@onField\"."]
    );
}

#[test]
#[ignore = "Suggestions are not yet supported"]
fn misspelled_directive_args_are_reported() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "{
          dog @skip(iff: true)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Unknown argument \"iff\" on directive \"@onField\". Did you mean \"if\"?"]
    );
}

#[test]
fn invalid_arg_name() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment invalidArgName on Dog {
          doesKnowCommand(unknown: true)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Unknown argument \"unknown\" on field \"Dog.doesKnowCommand\"."]
    );
}

#[test]
#[ignore = "Suggestions are not yet supported"]
fn misspelled_arg_name_is_reported() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment invalidArgName on Dog {
          doesKnowCommand(DogCommand: true)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Unknown argument \"DogCommand\" on field \"Dog.doesKnowCommand\". Did you mean \"dogCommand\"?"]
    );
}

#[test]
fn unknown_args_amongst_known_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "fragment oneGoodArgOneInvalidArg on Dog {
          doesKnowCommand(whoKnows: 1, dogCommand: SIT, unknown: true)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Unknown argument \"whoKnows\" on field \"Dog.doesKnowCommand\".",
            "Unknown argument \"unknown\" on field \"Dog.doesKnowCommand\"."
        ]
    );
}

#[test]
fn unknown_args_deeply() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownArgumentNames {}));
    let errors = test_operation_with_schema(
        "{
          dog {
            doesKnowCommand(unknown: true)
          }
          human {
            pet {
              ... on Dog {
                doesKnowCommand(unknown: true)
              }
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
            "Unknown argument \"unknown\" on field \"Dog.doesKnowCommand\".",
            "Unknown argument \"unknown\" on field \"Dog.doesKnowCommand\"."
        ]
    );
}
