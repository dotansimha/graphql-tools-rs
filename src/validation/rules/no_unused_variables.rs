use std::collections::HashMap;

use super::ValidationRule;
use crate::static_graphql::query::{OperationDefinition, VariableDefinition};
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// No unused fragments
///
/// A GraphQL operation is only valid if all variables defined by an operation
/// are used, either directly or within a spread fragment.
///
/// See https://spec.graphql.org/draft/#sec-All-Variables-Used
pub struct NoUnusedVariables;

struct NoUnusedVariablesHelper<'a> {
  variables_definitions: Vec<VariableDefinition>,
  error_context: ValidationErrorContext<'a>,
}

impl<'a> QueryVisitor<NoUnusedVariablesHelper<'a>> for NoUnusedVariables {
  fn enter_variable_definition(
    &self,
    _node: &VariableDefinition,
    _parent_operation: &OperationDefinition,
    _visitor_context: &mut NoUnusedVariablesHelper,
  ) {
    _visitor_context.variables_definitions.push(_node.clone());
  }

  fn leave_variable_definition(
    &self,
    _node: &VariableDefinition,
    _parent_operation: &OperationDefinition,
    _visitor_context: &mut NoUnusedVariablesHelper<'a>,
  ) {
    let variable_name_used = HashMap::new();
  }
}

impl<'a> NoUnusedVariablesHelper<'a> {
  fn new(validation_context: &'a ValidationContext<'a>) -> Self {
    NoUnusedVariablesHelper {
      variables_definitions: Vec::new(),
      error_context: ValidationErrorContext::new(validation_context),
    }
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
