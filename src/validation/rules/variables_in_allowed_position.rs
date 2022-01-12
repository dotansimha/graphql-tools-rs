use super::ValidationRule;
use crate::ast::ext::{AstWithVariables, ExtendedValue, TypeExtension};
use crate::ast::{
    AstTypeRef, PossibleInputType, TypeDefinitionExtension, TypeInfo, TypeInfoElementRef,
    TypeInfoQueryVisitor,
};
use crate::static_graphql::query::{Type, VariableDefinition};
use crate::static_graphql::schema::TypeDefinition;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Variables in allowed position
///
/// Variable usages must be compatible with the arguments they are passed to.
///
/// See https://spec.graphql.org/draft/#sec-All-Variable-Usages-are-Allowed
pub struct VariablesInAllowedPosition;

fn is_type_subtype_of(
    (maybe_sub_type, maybe_sub_type_schema_type): (&Type, &TypeDefinition),
    (super_type, super_type_schema_type): (&Type, &TypeDefinition),
) -> bool {
    if super_type.is_named_type()
        && maybe_sub_type.is_named_type()
        && maybe_sub_type.named_type().eq(&super_type.named_type())
    {
        return true;
    }

    if super_type.is_non_null_type() {
        if maybe_sub_type.is_non_null_type() {
            return is_type_subtype_of(
                (maybe_sub_type.inner_type(), maybe_sub_type_schema_type),
                (super_type.inner_type(), super_type_schema_type),
            );
        }

        return false;
    }

    if maybe_sub_type.is_non_null_type() {
        return is_type_subtype_of(
            (maybe_sub_type.inner_type(), maybe_sub_type_schema_type),
            (super_type, super_type_schema_type),
        );
    }

    if super_type.is_list_type() {
        if maybe_sub_type.is_list_type() {
            return is_type_subtype_of(
                (maybe_sub_type.inner_type(), maybe_sub_type_schema_type),
                (super_type.inner_type(), super_type_schema_type),
            );
        }

        return false;
    }

    if maybe_sub_type.is_list_type() {
        return false;
    }

    return super_type_schema_type.is_abstract_type()
        && (maybe_sub_type_schema_type.is_interface_type()
            || maybe_sub_type_schema_type.is_object_type());

    // Missing here: schema.isSubType(superType, maybeSubType)
}

fn is_variable_compatible(
    variable_definition: &VariableDefinition,
    variable_schema_type: &TypeDefinition,
    location_input_type: &PossibleInputType,
) -> bool {
    let var_type = &variable_definition.var_type;
    let var_default_value = &variable_definition.default_value;
    let location_type = location_input_type.get_type();
    let location_schema_type = location_input_type.get_schema_type_definition();

    if let Type::NonNullType(_t) = location_type {
        match var_type {
            Type::NonNullType(_t) => {}
            _ => {
                let has_non_null_variable_default_value =
                    var_default_value.is_some() && !var_default_value.as_ref().unwrap().is_null();
                let has_location_default_value = location_input_type.get_default_value().is_some();

                if !has_non_null_variable_default_value && !has_location_default_value {
                    return false;
                }

                if let Type::NonNullType(inner_type) = location_type {
                    return is_type_subtype_of(
                        (var_type, variable_schema_type),
                        (inner_type, &location_schema_type),
                    );
                }

                return is_type_subtype_of(
                    (var_type, variable_schema_type),
                    (location_type, &location_schema_type),
                );
            }
        }
    }

    return is_type_subtype_of(
        (var_type, variable_schema_type),
        (location_type, &location_schema_type),
    );
}

impl<'a> TypeInfoQueryVisitor<ValidationErrorContext<'a>> for VariablesInAllowedPosition {
    fn enter_operation_definition(
        &self,
        node: &crate::static_graphql::query::OperationDefinition,
        visitor_context: &mut ValidationErrorContext<'a>,
        _type_info: &TypeInfo,
    ) {
        let defined_variables = node.get_variables();
        let variables_in_use = node.get_variables_in_use(
            &visitor_context.ctx.fragments,
            visitor_context.ctx.type_info_registry.as_ref().unwrap(),
        );

        variables_in_use
            .iter()
            .for_each(|(usage_var_name, maybe_input_type_info)| {
                if let Some(TypeInfoElementRef::Ref(input_type_info)) = maybe_input_type_info {
                    if let Some(variable_definition) =
                        defined_variables.iter().find(|v| v.name.eq(usage_var_name))
                    {
                        if let Some(schema_base_type) =
                            visitor_context.ctx.find_schema_definition_by_name(
                                variable_definition.var_type.named_type(),
                            )
                        {
                            if !is_variable_compatible(
                                variable_definition,
                                schema_base_type,
                                input_type_info,
                            ) {
                                visitor_context.report_error(ValidationError {
                                    locations: vec![variable_definition.position],
                                    message: format!(
                                        "Variable \"${}\" of type \"{}\" used in position expecting type \"{}\".",
                                        usage_var_name, 
                                        variable_definition.var_type,
                                        input_type_info.get_type()
                                    ),
                                });
                            }
                        }
                    }
                }
            });
    }
}

impl ValidationRule for VariablesInAllowedPosition {
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
fn boolean_to_boolean() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($booleanArg: Boolean)
        {
          complicatedArgs {
            booleanArgField(booleanArg: $booleanArg)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn boolean_to_boolean_within_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "fragment booleanArgFrag on ComplicatedArgs {
          booleanArgField(booleanArg: $booleanArg)
        }
        query Query($booleanArg: Boolean)
        {
          complicatedArgs {
            ...booleanArgFrag
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);

    let errors = test_operation_with_schema(
        "query Query($booleanArg: Boolean)
      {
        complicatedArgs {
          ...booleanArgFrag
        }
      }
      fragment booleanArgFrag on ComplicatedArgs {
        booleanArgField(booleanArg: $booleanArg)
      }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn boolean_nonnull_to_boolean() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($nonNullBooleanArg: Boolean!)
        {
          complicatedArgs {
            booleanArgField(booleanArg: $nonNullBooleanArg)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn string_list_to_string_list() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringListVar: [String])
        {
          complicatedArgs {
            stringListArgField(stringListArg: $stringListVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn string_list_nonnull_to_string_list() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringListVar: [String!])
        {
          complicatedArgs {
            stringListArgField(stringListArg: $stringListVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn string_to_string_list_in_item_position() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringVar: String)
        {
          complicatedArgs {
            stringListArgField(stringListArg: [$stringVar])
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn string_nonnull_to_string_list_in_item_position() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringVar: String!)
        {
          complicatedArgs {
            stringListArgField(stringListArg: [$stringVar])
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn complexinput_to_complexinput() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($complexVar: ComplexInput)
        {
          complicatedArgs {
            complexArgField(complexArg: $complexVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn complexinput_to_complexinput_in_field_position() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($boolVar: Boolean = false)
        {
          complicatedArgs {
            complexArgField(complexArg: { requiredArg: $boolVar })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn boolean_nonnull_to_boolean_nonnull_in_directive() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($boolVar: Boolean!)
        {
          dog @include(if: $boolVar)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn int_to_int_nonnull() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($intArg: Int) {
          complicatedArgs {
            nonNullIntArgField(nonNullIntArg: $intArg)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$intArg\" of type \"Int\" used in position expecting type \"Int!\"."
    ])
}


#[test]
fn int_to_int_nonnull_within_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "fragment nonNullIntArgFieldFrag on ComplicatedArgs {
          nonNullIntArgField(nonNullIntArg: $intArg)
        }
        query Query($intArg: Int) {
          complicatedArgs {
            ...nonNullIntArgFieldFrag
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$intArg\" of type \"Int\" used in position expecting type \"Int!\"."
    ])
}

#[test]
fn int_to_int_nonnull_within_nested_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "fragment outerFrag on ComplicatedArgs {
          ...nonNullIntArgFieldFrag
        }
        fragment nonNullIntArgFieldFrag on ComplicatedArgs {
          nonNullIntArgField(nonNullIntArg: $intArg)
        }
        query Query($intArg: Int) {
          complicatedArgs {
            ...outerFrag
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$intArg\" of type \"Int\" used in position expecting type \"Int!\"."
    ])
}

#[test]
fn string_over_boolean() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringVar: String) {
          complicatedArgs {
            booleanArgField(booleanArg: $stringVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$stringVar\" of type \"String\" used in position expecting type \"Boolean\"."
    ])
}

#[test]
fn string_over_string_list() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringVar: String) {
          complicatedArgs {
            stringListArgField(stringListArg: $stringVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$stringVar\" of type \"String\" used in position expecting type \"[String]\"."
    ])
}

#[test]
fn boolean_to_boolean_nonnull_in_directive() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($boolVar: Boolean) {
          dog @include(if: $boolVar)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$boolVar\" of type \"Boolean\" used in position expecting type \"Boolean!\"."
    ])
}

#[test]
fn string_to_boolean_nonnull_in_directive() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringVar: String) {
          dog @include(if: $stringVar)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$stringVar\" of type \"String\" used in position expecting type \"Boolean!\"."
    ])
}

#[test]
fn string_list_to_string_nonnull_list() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($stringListVar: [String])
        {
          complicatedArgs {
            stringListNonNullArgField(stringListNonNullArg: $stringListVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$stringListVar\" of type \"[String]\" used in position expecting type \"[String!]\"."
    ])
}

#[test]
fn int_to_int_non_null_with_null_default_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($intVar: Int = null) {
          complicatedArgs {
            nonNullIntArgField(nonNullIntArg: $intVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Variable \"$intVar\" of type \"Int\" used in position expecting type \"Int!\"."
    ])
}

#[test]
fn int_to_int_non_null_with_default_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($intVar: Int = 1) {
          complicatedArgs {
            nonNullIntArgField(nonNullIntArg: $intVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn int_to_int_non_null_where_argument_with_default_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($intVar: Int) {
          complicatedArgs {
            nonNullFieldWithDefault(nonNullIntArg: $intVar)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn boolean_to_boolean_non_null_with_default_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesInAllowedPosition {}));
    let errors = test_operation_with_schema(
        "query Query($boolVar: Boolean = false) {
          dog @include(if: $boolVar)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}
