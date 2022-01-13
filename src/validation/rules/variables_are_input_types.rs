use super::ValidationRule;
use crate::ast::{AstTypeRef, TypeDefinitionExtension, TypeInfoQueryVisitor};
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Variables are input types
///
/// A GraphQL operation is only valid if all the variables it defines are of
/// input types (scalar, enum, or input object).
///
/// See https://spec.graphql.org/draft/#sec-Variables-Are-Input-Types
pub struct VariablesAreInputTypes;

impl<'a> TypeInfoQueryVisitor<ValidationErrorContext<'a>> for VariablesAreInputTypes {
    fn enter_variable_definition(
        &self,
        _node: &crate::static_graphql::query::VariableDefinition,
        _visitor_context: &mut ValidationErrorContext<'a>,
        _type_info: &crate::ast::TypeInfo,
    ) {
        if let Some(schema_type) = _visitor_context
            .ctx
            .find_schema_definition_by_name(_node.var_type.named_type())
        {
            if !schema_type.is_input_type() {
                _visitor_context.report_error(ValidationError {
                    message: format!(
                        "Variable \"${}\" cannot be non-input type \"{}\".",
                        _node.name, _node.var_type
                    ),
                    locations: vec![_node.position],
                })
            }
        }
    }
}

impl ValidationRule for VariablesAreInputTypes {
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
fn unknown_types_are_ignored() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesAreInputTypes {}));
    let errors = test_operation_with_schema(
        "
        query Foo($a: Unknown, $b: [[Unknown!]]!) {
          field(a: $a, b: $b)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn input_types_are_valid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesAreInputTypes {}));
    let errors = test_operation_with_schema(
        "
        query Foo($a: String, $b: [Boolean!]!, $c: ComplexInput) {
          field(a: $a, b: $b, c: $c)
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn output_types_are_invalid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(VariablesAreInputTypes {}));
    let errors = test_operation_with_schema(
        "
       query Foo($a: Dog, $b: [[CatOrDog!]]!, $c: Pet) {
        field(a: $a, b: $b, c: $c)
      }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages,
        vec![
            "Variable \"$a\" cannot be non-input type \"Dog\".",
            "Variable \"$b\" cannot be non-input type \"[[CatOrDog!]]!\".",
            "Variable \"$c\" cannot be non-input type \"Pet\".",
        ]
    );
}
