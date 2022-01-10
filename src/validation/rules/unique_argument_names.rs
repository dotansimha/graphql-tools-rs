use std::collections::HashMap;

use graphql_parser::Pos;

use super::ValidationRule;
use crate::static_graphql::query::Value;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// Unique argument names
///
/// A GraphQL field or directive is only valid if all supplied arguments are
/// uniquely named.
///
/// See https://spec.graphql.org/draft/#sec-Argument-Names
pub struct UniqueArgumentNames;

fn collect_from_arguments(
    reported_position: Pos,
    arguments: &Vec<(String, Value)>,
) -> HashMap<String, Vec<Pos>> {
    let mut found_args = HashMap::<String, Vec<Pos>>::new();

    for (arg_name, _arg_value) in arguments {
        found_args
            .entry(arg_name.clone())
            .or_insert(vec![])
            .push(reported_position);
    }

    found_args
}

impl<'a> QueryVisitor<ValidationErrorContext<'a>> for UniqueArgumentNames {
    fn enter_field(
        &self,
        field: &crate::static_graphql::query::Field,
        visitor_context: &mut ValidationErrorContext<'a>,
    ) {
        let found_args = collect_from_arguments(field.position, &field.arguments);

        found_args.iter().for_each(|(arg_name, positions)| {
            if positions.len() > 1 {
                visitor_context.report_error(ValidationError {
                    message: format!("There can be only one argument named \"{}\".", arg_name),
                    locations: positions.clone(),
                })
            }
        });
    }

    fn enter_directive(
        &self,
        directive: &crate::static_graphql::query::Directive,
        visitor_context: &mut ValidationErrorContext<'a>,
    ) {
        let found_args = collect_from_arguments(directive.position, &directive.arguments);

        found_args.iter().for_each(|(arg_name, positions)| {
            if positions.len() > 1 {
                visitor_context.report_error(ValidationError {
                    message: format!("There can be only one argument named \"{}\".", arg_name),
                    locations: positions.clone(),
                })
            }
        });
    }
}

impl ValidationRule for UniqueArgumentNames {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = ValidationErrorContext::new(&ctx);
        self.visit_document(&ctx.operation.clone(), &mut helper);

        helper.errors
    }
}

#[test]
fn no_arguments_on_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn no_arguments_on_directive() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field @directive
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn argument_on_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field(arg: \"value\")
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn argument_on_directive() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field @directive(arg: \"value\")
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn same_argument_on_two_fields() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          one: field(arg: \"value\")
          two: field(arg: \"value\")
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn same_argument_on_field_and_directive() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field(arg: \"value\") @directive(arg: \"value\")
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn same_argument_on_two_directives() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field @directive1(arg: \"value\") @directive2(arg: \"value\")
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn multiple_field_arguments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field(arg1: \"value\", arg2: \"value\", arg3: \"value\")
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn multiple_directive_argument() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field @directive(arg1: \"value\", arg2: \"value\", arg3: \"value\")
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn duplicate_field_arguments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field(arg1: \"value\", arg1: \"value\")
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one argument named \"arg1\"."]
    );
}

#[test]
fn many_duplicate_field_arguments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field(arg1: \"value\", arg1: \"value\", arg1: \"value\")
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one argument named \"arg1\"."]
    );
}

#[test]
fn duplicate_directive_arguments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field @directive(arg1: \"value\", arg1: \"value\")
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one argument named \"arg1\"."]
    );
}

#[test]
fn many_duplicate_directive_arguments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(UniqueArgumentNames {}));
    let errors = test_operation_without_schema(
        "{
          field @directive(arg1: \"value\", arg1: \"value\", arg1: \"value\")
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["There can be only one argument named \"arg1\"."]
    );
}
