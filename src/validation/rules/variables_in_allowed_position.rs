use super::ValidationRule;
use crate::ast::ext::{AstWithVariables, ExtendedValue};
use crate::ast::{
    AstTypeRef, PossibleInputType, TypeInfo, TypeInfoElementRef, TypeInfoQueryVisitor,
};
use crate::static_graphql::query::{Value, VariableDefinition, Type};
use crate::static_graphql::schema::TypeDefinition;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Variables in allowed position
///
/// Variable usages must be compatible with the arguments they are passed to.
///
/// See https://spec.graphql.org/draft/#sec-All-Variable-Usages-are-Allowed
pub struct VariablesInAllowedPosition;

fn is_variable_compatible(
    variable_definition: &VariableDefinition,
    location_input_type: PossibleInputType,
    location_default_value: Option<Value>,
) -> bool {
    let var_type = &variable_definition.var_type;
    let var_default_value = &variable_definition.default_value;
    let location_type = location_input_type.get_type();

    if let Type::NonNullType(_t) = location_type {
      match var_type {
        Type::NonNullType(_t) => {},
        _ => {
          let has_non_null_variable_default_value = var_default_value.is_some() && !var_default_value.as_ref().unwrap().is_null();
          let has_location_default_value = location_default_value.is_some();

          if !has_non_null_variable_default_value && !has_location_default_value {
            return false;
          }
        },
      }
    }

    false
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
                          
                            println!(
                                "enter_operation_definition, variable_definition.var_type: {}, usage_var_name: {},  schema_base_type: {:?}, input_type_info: {:?}",
                                variable_definition.var_type,
                                usage_var_name,
                                schema_base_type,
                                input_type_info
                            )

                            // input_type_info.
                            // if !is_variable_compatible(schema_type, defined_variable) {}
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
        "query Query($booleanArg: Boolean!)
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
