use std::collections::HashSet;

use super::ValidationRule;
use crate::ast::AstNodeWithName;
use crate::static_graphql::query::{OperationDefinition, VariableDefinition};
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{
    ast::{ext::AstWithVariables, QueryVisitor},
    validation::utils::ValidationContext,
};

/// No unused fragments
///
/// A GraphQL operation is only valid if all variables defined by an operation
/// are used, either directly or within a spread fragment.
///
/// See https://spec.graphql.org/draft/#sec-All-Variables-Used
pub struct NoUnusedVariables;

struct NoUnusedVariablesHelper<'a> {
    variables_definitions: HashSet<String>,
    error_context: ValidationErrorContext<'a>,
}

impl<'a> NoUnusedVariablesHelper<'a> {
    fn new(validation_context: &'a ValidationContext<'a>) -> Self {
        NoUnusedVariablesHelper {
            variables_definitions: HashSet::new(),
            error_context: ValidationErrorContext::new(validation_context),
        }
    }
}

impl<'a> QueryVisitor<NoUnusedVariablesHelper<'a>> for NoUnusedVariables {
    fn enter_variable_definition(
        &self,
        node: &VariableDefinition,
        _parent_operation: &OperationDefinition,
        visitor_context: &mut NoUnusedVariablesHelper<'a>,
    ) {
        visitor_context
            .variables_definitions
            .insert(node.name.clone());
    }

    fn enter_operation_definition(
        &self,
        _node: &OperationDefinition,
        visitor_context: &mut NoUnusedVariablesHelper<'a>,
    ) {
        visitor_context.variables_definitions.clear();
    }

    fn leave_operation_definition(
        &self,
        node: &OperationDefinition,
        visitor_context: &mut NoUnusedVariablesHelper<'a>,
    ) {
        let in_use = node.get_variables_in_use(&visitor_context.error_context.ctx.fragments);

        visitor_context
            .variables_definitions
            .iter()
            .filter(|variable_name| !in_use.contains(variable_name.clone()))
            .for_each(|unused_variable_name| {
                visitor_context.error_context.report_error(ValidationError {
                    locations: vec![],
                    message: match node.node_name() {
                        Some(name) => format!(
                            "Variable \"${}\" is never used in operation \"{}\".",
                            unused_variable_name, name
                        ),
                        None => format!("Variable \"${}\" is never used.", unused_variable_name),
                    },
                });
            });
    }
}

impl ValidationRule for NoUnusedVariables {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = NoUnusedVariablesHelper::new(&ctx);
        self.visit_document(&ctx.operation.clone(), &mut helper);

        helper.error_context.errors
    }
}

#[test]
fn use_all_variables() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_without_schema(
        "query ($a: String, $b: String, $c: String) {
        field(a: $a, b: $b, c: $c)
      }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn use_all_variables_deeply() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String, $b: String, $c: String) {
      field(a: $a) {
        field(b: $b) {
          field(c: $c)
        }
      }
    }
  ",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn use_all_variables_deeply_in_inline_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_without_schema(
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
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn use_all_variables_in_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_without_schema(
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
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn variables_used_by_fragment_in_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_without_schema(
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
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn variables_used_by_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String) {
      ...FragA
    }
    fragment FragA on Type {
      field(a: $a) {
        ...FragA
      }
    }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn variables_not_used() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedVariables {}));
    let errors = test_operation_without_schema(
        "query ($a: String, $b: String, $c: String) {
          field(a: $a, b: $b)
        }",
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
    let errors = test_operation_without_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(b: $b)
        }",
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
    let errors = test_operation_without_schema(
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
    let errors = test_operation_without_schema(
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
    let errors = test_operation_without_schema(
        "query Foo($b: String) {
          ...FragA
        }
        fragment FragA on Type {
          field(a: $a)
        }
        fragment FragB on Type {
          field(b: $b)
        }",
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
    let errors = test_operation_without_schema(
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
        &mut plan,
    );

    let messages = get_messages(&errors);

    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&&"Variable \"$b\" is never used in operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$a\" is never used in operation \"Bar\".".to_owned()));
}
