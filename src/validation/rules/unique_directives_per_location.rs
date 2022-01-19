use std::collections::HashSet;

use super::ValidationRule;
use crate::static_graphql::query::{
    Directive, Field, FragmentDefinition, InlineFragment, OperationDefinition,
};
use crate::static_graphql::schema::DirectiveLocation;
use crate::{
    ast::{visit_document, OperationVisitor, OperationVisitorContext},
    validation::utils::{ValidationError, ValidationErrorContext},
};

/// Unique directive names per location
///
/// A GraphQL document is only valid if all non-repeatable directives at
/// a given location are uniquely named.
///
/// See  https://spec.graphql.org/draft/#sec-Directives-Are-Unique-Per-Location
pub struct UniqueDirectivesPerLocation {
    recent_location: Option<DirectiveLocation>,
}

impl UniqueDirectivesPerLocation {
    pub fn new() -> Self {
        UniqueDirectivesPerLocation {
            recent_location: None,
        }
    }
}

impl<'a> OperationVisitor<'a, ValidationErrorContext> for UniqueDirectivesPerLocation {
    fn enter_operation_definition(
        &mut self,
        ctx: &mut OperationVisitorContext<'a>,
        err_ctx: &mut ValidationErrorContext,
        op: &OperationDefinition,
    ) {
        check_duplicate_directive(ctx, err_ctx);
    }
}

fn check_duplicate_directive<'a>(
    ctx: &mut OperationVisitorContext<'a>,
    err_context: &mut ValidationErrorContext,
    directives: Vec<Directive>,
) {
    let mut exists = HashSet::new();

    for directive in directives {
        if let Some(meta_directive) = ctx.directives.get(&directive.name) {
            if !meta_directive.repeatable {
                if exists.contains(&directive.name) {
                    err_context.report_error(ValidationError {
                        locations: vec![directive.position],
                        message: format!("Duplicate directive \"{}\"", &directive.name),
                    });
                    continue;
                }
                exists.insert(directive.name.clone());
            }
        }
    }
}

impl ValidationRule for UniqueDirectivesPerLocation {
    fn validate<'a>(
        &self,
        ctx: &'a mut OperationVisitorContext,
        error_collector: &mut ValidationErrorContext,
    ) {
        visit_document(
            &mut UniqueDirectivesPerLocation::new(),
            &ctx.operation,
            ctx,
            error_collector,
        );
    }
}

#[test]
fn no_directives() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        " fragment Test on Type {
            field
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn unique_directives_in_different_locations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        "fragment Test on Type @directiveA {
            field @directiveB
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn unique_directives_in_same_location() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        "fragment Test on Type @directiveA @directiveB {
            field @directiveA @directiveB
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn same_directives_in_different_locations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        "fragment Test on Type @directiveA {
            field @directiveA
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn same_directives_in_similar_locations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        "fragment Test on Type {
            field @directive
            field @directive
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn repeatable_directives_in_same_location() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        "fragment Test on Type @repeatable @repeatable {
            field @repeatable @repeatable
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn unknown_directives_must_be_ignored() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        "fragment Test on Type @repeatable @repeatable {
            field @repeatable @repeatable
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn duplicate_directives_in_one_location() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation::new()));
    let errors = test_operation_with_schema(
        "fragment Test on Type {
            field @directive @directive
          }",
        &TEST_SCHEMA,
        &mut plan,
    );
    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
}
