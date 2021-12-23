use super::ValidationRule;
// use crate::static_graphql::query::*;
use crate::{
    ast::{ext::*, get_named_type, TypeInfoQueryVisitor},
    validation::utils::{ValidationContext, ValidationError, ValidationErrorContext},
};

/// Leaf Field Selections
///
/// Field selections on scalars or enums are never allowed, because they are the leaf nodes of any GraphQL operation.
///
/// https://spec.graphql.org/draft/#sec-Leaf-Field-Selections
pub struct LeafFieldSelections;

impl TypeInfoQueryVisitor<ValidationErrorContext<'_>> for LeafFieldSelections {
    fn enter_field(
        &self,
        _node: &crate::static_graphql::query::Field,
        _visitor_context: &mut ValidationErrorContext<'_>,
        _type_info: &mut crate::ast::TypeInfo,
    ) {
        if let Some(field_type) = _type_info.get_type() {
            let named_type = get_named_type(&field_type);
            let schema_type = _visitor_context
                .ctx
                .type_info_registry
                .as_ref()
                .unwrap()
                .type_by_name
                .get(&named_type)
                .unwrap();
            let selection_set = &_node.selection_set;

            if schema_type.is_leaf_type() {
                if selection_set.items.len() > 0 {
                    _visitor_context.report_error(ValidationError {
                        locations: vec![_node.position],
                        message: format!(
                            "Field \"{}\" must not have a selection since type \"{}\" has no subfields.",
                            _node.name,
                            schema_type.name()
                        ),
                    });
                }
            } else {
                if selection_set.items.len() == 0 {
                    _visitor_context.report_error(ValidationError {
                    locations: vec![_node.position],
                    message: format!(
                        "Field \"{}\" of type \"{}\" must have a selection of subfields. Did you mean \"{} {{ ... }}\"?",
                        _node.name,
                        field_type,
                        _node.name
                    ),
                });
                }
            }
        }
    }
}

impl ValidationRule for LeafFieldSelections {
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
fn valid_scalar_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "fragment scalarSelection on Dog {
          barks
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn object_type_missing_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "query directQueryOnObjectWithoutSubFields {
          human
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"human\" of type \"Human\" must have a selection of subfields. Did you mean \"human { ... }\"?"]
    );
}

#[test]
fn interface_type_missing_selection() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "{
          human { pets }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"pets\" of type \"[Pet]\" must have a selection of subfields. Did you mean \"pets { ... }\"?"]
    );
}

#[test]
fn selection_not_allowed_on_scalar() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "fragment scalarSelectionsNotAllowedOnBoolean on Dog {
          barks { sinceWhen }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"barks\" must not have a selection since type \"Boolean\" has no subfields."]
    );
}

#[test]
fn selection_not_allowed_on_enum() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "fragment scalarSelectionsNotAllowedOnEnum on Cat {
          furColor { inHexDec }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"furColor\" must not have a selection since type \"FurColor\" has no subfields."]
    );
}

#[test]
fn scalar_selection_not_allowed_with_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "fragment scalarSelectionsNotAllowedWithArgs on Dog {
          doesKnowCommand(dogCommand: SIT) { sinceWhen }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"doesKnowCommand\" must not have a selection since type \"Boolean\" has no subfields."]
    );
}

#[test]
fn scalar_selection_not_allowed_with_directives() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "fragment scalarSelectionsNotAllowedWithDirectives on Dog {
          name @include(if: true) { isAlsoHumanName }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"name\" must not have a selection since type \"String\" has no subfields."]
    );
}

#[test]
fn scalar_selection_not_allowed_with_directives_and_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LeafFieldSelections {}));
    let errors = test_operation_with_schema(
        "fragment scalarSelectionsNotAllowedWithDirectivesAndArgs on Dog {
          doesKnowCommand(dogCommand: SIT) @include(if: true) { sinceWhen }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"doesKnowCommand\" must not have a selection since type \"Boolean\" has no subfields."]
    );
}
