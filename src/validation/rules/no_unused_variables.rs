use super::ValidationRule;
use crate::ast::{visit_document, AstNodeWithName, OperationVisitor, OperationVisitorContext};
use crate::static_graphql::query::OperationDefinition;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::ext::AstWithVariables, validation::utils::ValidationContext};

/// No unused fragments
///
/// A GraphQL operation is only valid if all variables defined by an operation
/// are used, either directly or within a spread fragment.
///
/// See https://spec.graphql.org/draft/#sec-All-Variables-Used
pub struct NoUnusedVariables;

impl<'a> OperationVisitor<'a, ValidationErrorContext> for NoUnusedVariables {
    fn leave_operation_definition(
        &mut self,
        visitor_context: &mut crate::ast::OperationVisitorContext<ValidationErrorContext>,
        operation: &OperationDefinition,
    ) {
        let variables = operation.get_variables();
        let in_use = operation.get_variables_in_use(&visitor_context.known_fragments);

        variables
            .iter()
            .filter(|variable_name| !in_use.contains(&variable_name.name))
            .for_each(|unused_variable_name| {
                visitor_context.user_context.report_error(ValidationError {
                    locations: vec![],
                    message: match operation.node_name() {
                        Some(name) => format!(
                            "Variable \"${}\" is never used in operation \"{}\".",
                            unused_variable_name.name, name
                        ),
                        None => {
                            format!("Variable \"${}\" is never used.", unused_variable_name.name)
                        }
                    },
                });
            });
    }
}

impl ValidationRule for NoUnusedVariables {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut visitor_helper = ValidationErrorContext::new();

        visit_document(
            &mut NoUnusedVariables {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut visitor_helper, &ctx.operation, &ctx.schema),
        );

        visitor_helper.errors
    }
}

#[test]
fn use_all_variables() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query ($a: String, $b: String, $c: String) {
        field(a: $a, b: $b, c: $c)
      }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn use_all_variables_deeply() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
      field(a: $a) {
        field(b: $b) {
          field(c: $c)
        }
      }
    }
  ",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn use_all_variables_deeply_in_inline_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        " query Foo($a: String, $b: String, $c: String) {
      ... on Type {
        field(a: $a) {
          field(b: $b) {
            ... on Type {
              field(c: $c)
            }
          }
        }
      }
    }
  ",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn use_all_variables_in_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
      ...FragA
    }
    fragment FragA on Type {
      field(a: $a) {
        ...FragB
      }
    }
    fragment FragB on Type {
      field(b: $b) {
        ...FragC
      }
    }
    fragment FragC on Type {
      field(c: $c)
    }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn variables_used_by_fragment_in_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String) {
      ...FragA
    }
    query Bar($b: String) {
      ...FragB
    }
    fragment FragA on Type {
      field(a: $a)
    }
    fragment FragB on Type {
      field(b: $b)
    }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn variables_used_by_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String) {
      ...FragA
    }
    fragment FragA on Type {
      field(a: $a) {
        ...FragA
      }
    }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn variables_not_used() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query ($a: String, $b: String, $c: String) {
          field(a: $a, b: $b)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 1);
    assert!(messages.contains(&&"Variable \"$c\" is never used.".to_owned()));
}

#[test]
fn multiple_variables_not_used() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(b: $b)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&&"Variable \"$a\" is never used in operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$c\" is never used in operation \"Foo\".".to_owned()));
}

#[test]
fn variables_not_used_in_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
          ...FragA
        }
        fragment FragA on Type {
          field(a: $a) {
            ...FragB
          }
        }
        fragment FragB on Type {
          field(b: $b) {
            ...FragC
          }
        }
        fragment FragC on Type {
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 1);
    assert!(messages.contains(&&"Variable \"$c\" is never used in operation \"Foo\".".to_owned()));
}

#[test]
fn multiple_variables_not_used_in_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
          ...FragA
        }
        fragment FragA on Type {
          field {
            ...FragB
          }
        }
        fragment FragB on Type {
          field(b: $b) {
            ...FragC
          }
        }
        fragment FragC on Type {
          field
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&&"Variable \"$a\" is never used in operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$c\" is never used in operation \"Foo\".".to_owned()));
}

#[test]
fn variables_not_used_by_unreferences_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($b: String) {
          ...FragA
        }
        fragment FragA on Type {
          field(a: $a)
        }
        fragment FragB on Type {
          field(b: $b)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 1);
    assert!(messages.contains(&&"Variable \"$b\" is never used in operation \"Foo\".".to_owned()));
}

#[test]
fn variables_not_used_by_fragment_used_by_other_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($b: String) {
          ...FragA
        }
        query Bar($a: String) {
          ...FragB
        }
        fragment FragA on Type {
          field(a: $a)
        }
        fragment FragB on Type {
          field(b: $b)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&&"Variable \"$b\" is never used in operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$a\" is never used in operation \"Bar\".".to_owned()));
}

#[test]
fn should_also_check_directives_usage() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query foo($skip: Boolean!) {
          field @skip(if: $skip)
        }
        ",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn nested_variable_should_work_as_well() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_with_schema(
        "query foo($t: Boolean!) {
          field(boop: { test: $t})
        }
        ",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}
