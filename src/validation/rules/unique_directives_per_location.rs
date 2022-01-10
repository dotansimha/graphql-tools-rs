use super::ValidationRule;
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
pub struct UniqueDirectivesPerLocation;

impl UniqueDirectivesPerLocation {
    pub fn new() -> Self {
        UniqueDirectivesPerLocation
    }
}

impl<'a> OperationVisitor<'a, ValidationErrorContext> for UniqueDirectivesPerLocation {
    // TODO: Finish me
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
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

    let mut plan = create_plan_from_rule(Box::new(UniqueDirectivesPerLocation {}));
    let errors = test_operation_with_schema(
        "fragment Test on Type {
            field @directive @directive
          }",
        &TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 1);
}
