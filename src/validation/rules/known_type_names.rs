use super::ValidationRule;
use crate::ast::ext::AstTypeRef;
use crate::ast::TypeInfoQueryVisitor;
use crate::static_graphql::query::TypeCondition;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};

/// Known type names
///
/// A GraphQL document is only valid if referenced types (specifically
/// variable definitions and fragment conditions) are defined by the type schema.
///
/// See https://spec.graphql.org/draft/#sec-Fragment-Spread-Type-Existence
pub struct KnownTypeNames;

impl<'a> TypeInfoQueryVisitor<ValidationErrorContext<'a>> for KnownTypeNames {
    fn enter_fragment_definition(
        &self,
        _node: &crate::static_graphql::query::FragmentDefinition,
        _visitor_context: &mut ValidationErrorContext<'a>,
    ) {
        let TypeCondition::On(fragment_type_name) = &_node.type_condition;

        if let None = _visitor_context
            .ctx
            .find_schema_definition_by_name(fragment_type_name.clone())
        {
            _visitor_context.errors.push(ValidationError {
                locations: vec![_node.position],
                message: format!("Unknown type \"{}\".", fragment_type_name),
            });
        }
    }

    fn enter_inline_fragment(
        &self,
        _node: &crate::static_graphql::query::InlineFragment,
        _visitor_context: &mut ValidationErrorContext<'a>,
        _type_info: &mut crate::ast::TypeInfo,
    ) {
        if let Some(TypeCondition::On(fragment_type_name)) = &_node.type_condition {
            if let None = _visitor_context
                .ctx
                .find_schema_definition_by_name(fragment_type_name.clone())
            {
                _visitor_context.errors.push(ValidationError {
                    locations: vec![_node.position],
                    message: format!("Unknown type \"{}\".", fragment_type_name),
                });
            }
        }
    }

    fn enter_variable_definition(
        &self,
        _node: &crate::static_graphql::query::VariableDefinition,
        _parent_operation: &crate::static_graphql::query::OperationDefinition,
        _visitor_context: &mut ValidationErrorContext<'a>,
        _type_info: &mut crate::ast::TypeInfo,
    ) {
        let base_type = _node.var_type.named_type();

        if let None = _visitor_context
            .ctx
            .find_schema_definition_by_name(base_type.clone())
        {
            _visitor_context.errors.push(ValidationError {
                locations: vec![_node.position],
                message: format!("Unknown type \"{}\".", base_type),
            });
        }
    }
}

impl ValidationRule for KnownTypeNames {
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
fn known_type_names_are_valid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownTypeNames {}));
    let errors = test_operation_with_schema(
        "
        query Foo(
          $var: String
          $required: [Int!]!
          $introspectionType: __EnumValue
        ) {
          user(id: 4) {
            pets { ... on Pet { name }, ...PetFields, ... { name } }
          }
        }
        fragment PetFields on Pet {
          name
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn unknown_type_names_are_invalid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownTypeNames {}));
    let errors = test_operation_with_schema(
        "query Foo($var: [JumbledUpLetters!]!) {
          user(id: 4) {
            name
            pets { ... on Badger { name }, ...PetFields }
          }
        }

        fragment PetFields on Peat {
          name
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages,
        vec![
            "Unknown type \"JumbledUpLetters\".",
            "Unknown type \"Badger\".",
            "Unknown type \"Peat\"."
        ]
    );
}
