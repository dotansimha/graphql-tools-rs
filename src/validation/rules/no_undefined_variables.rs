use super::ValidationRule;
use crate::ast::AstNodeWithName;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{
    ast::{ext::AstWithVariables, QueryVisitor},
    validation::utils::ValidationContext,
};
use std::any::Any;
use std::collections::HashSet;

/// No undefined variables
///
/// A GraphQL operation is only valid if all variables encountered, both directly
/// and via fragment spreads, are defined by that operation.
///
/// See https://spec.graphql.org/draft/#sec-All-Variable-Uses-Defined
pub struct NoUndefinedVariables;

struct NoUndefinedVariablesHelper<'a> {
    error_ctx: ValidationErrorContext<'a>,
    current_op_variables: HashSet<String>,
}

impl<'a> NoUndefinedVariablesHelper<'a> {
    fn new(error_ctx: ValidationErrorContext<'a>) -> Self {
        Self {
            error_ctx,
            current_op_variables: HashSet::new(),
        }
    }
}

impl<'a> QueryVisitor<NoUndefinedVariablesHelper<'a>> for NoUndefinedVariables {
    fn enter_variable_definition(
        &self,
        node: &crate::static_graphql::query::VariableDefinition,
        _parent_operation: &crate::static_graphql::query::OperationDefinition,
        visitor_context: &mut NoUndefinedVariablesHelper<'a>,
    ) {
        visitor_context
            .current_op_variables
            .insert(node.name.clone());
    }

    fn enter_operation_definition(
        &self,
        _node: &crate::static_graphql::query::OperationDefinition,
        visitor_context: &mut NoUndefinedVariablesHelper<'a>,
    ) {
        visitor_context.current_op_variables.clear();
    }

    fn leave_operation_definition(
        &self,
        node: &crate::static_graphql::query::OperationDefinition,
        visitor_context: &mut NoUndefinedVariablesHelper<'a>,
    ) {
        let in_use = node.get_variables_in_use(
            &visitor_context.error_ctx.ctx.fragments,
            visitor_context
                .error_ctx
                .ctx
                .type_info_registry
                .as_ref()
                .unwrap(),
        );

        in_use.iter().for_each(|(v, _attrs)| {
            if !visitor_context.current_op_variables.contains(v) {
                visitor_context.error_ctx.report_error(ValidationError {
                    message: match node.node_name() {
                        Some(name) => format!(
                            "Variable \"${}\" is not defined by operation \"{}\".",
                            v, name
                        ),
                        None => format!("Variable \"${}\" is not defined.", v),
                    },
                    locations: vec![],
                })
            }
        });
    }
}

impl ValidationRule for NoUndefinedVariables {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let error_ctx = ValidationErrorContext::new(&ctx);
        let mut helper = NoUndefinedVariablesHelper::new(error_ctx);

        self.visit_document(&ctx.operation.clone(), &mut helper);

        helper.error_ctx.errors
    }
}

#[test]
fn all_variables_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(a: $a, b: $b, c: $c)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn all_variables_deeply_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(a: $a) {
            field(b: $b) {
              field(c: $c)
            }
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn all_variables_deeply_in_inline_fragments_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String, $b: String, $c: String) {
          ... on Type {
            field(a: $a) {
              field(b: $b) {
                ... on Type {
                  field(c: $c)
                }
              }
            }
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn all_variables_in_fragments_deeply_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_within_single_fragment_defined_in_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String) {
          ...FragA
        }
        query Bar($a: String) {
          ...FragA
        }
        fragment FragA on Type {
          field(a: $a)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_within_fragments_defined_in_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_within_recursive_fragment_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_not_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(a: $a, b: $b, c: $c, d: $d)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Variable \"$d\" is not defined by operation \"Foo\"."]
    );
}

#[test]
fn variable_not_defined_by_un_named_query() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "{
          field(a: $a)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Variable \"$a\" is not defined."]);
}

#[test]
fn multiple_variables_not_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($b: String) {
          field(a: $a, b: $b, c: $c)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&&"Variable \"$a\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$c\" is not defined by operation \"Foo\".".to_owned()));
}

#[test]
fn variable_in_fragment_not_defined_by_un_named_query() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "{
          ...FragA
        }
        fragment FragA on Type {
          field(a: $a)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Variable \"$a\" is not defined.",]);
}

#[test]
fn variable_in_fragment_not_defined_by_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String, $b: String) {
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Variable \"$c\" is not defined by operation \"Foo\"."]
    );
}

#[test]
fn multiple_variables_in_fragments_not_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($b: String) {
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert!(messages.contains(&&"Variable \"$c\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$a\" is not defined by operation \"Foo\".".to_owned()));
}

#[test]
fn single_variable_in_fragment_not_defined_by_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($a: String) {
          ...FragAB
        }
        query Bar($a: String) {
          ...FragAB
        }
        fragment FragAB on Type {
          field(a: $a, b: $b)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Variable \"$b\" is not defined by operation \"Foo\".",
            "Variable \"$b\" is not defined by operation \"Bar\"."
        ]
    );
}

#[test]
fn variables_in_fragment_not_defined_by_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($b: String) {
          ...FragAB
        }
        query Bar($a: String) {
          ...FragAB
        }
        fragment FragAB on Type {
          field(a: $a, b: $b)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Variable \"$a\" is not defined by operation \"Foo\".",
            "Variable \"$b\" is not defined by operation \"Bar\"."
        ]
    );
}

#[test]
fn variable_in_fragment_used_by_other_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
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
    assert_eq!(
        messages,
        vec![
            "Variable \"$a\" is not defined by operation \"Foo\".",
            "Variable \"$b\" is not defined by operation \"Bar\"."
        ]
    );
}
#[test]
fn multiple_undefined_variables_produce_multiple_errors() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_without_schema(
        "query Foo($b: String) {
          ...FragAB
        }
        query Bar($a: String) {
          ...FragAB
        }
        fragment FragAB on Type {
          field1(a: $a, b: $b)
          ...FragC
          field3(a: $a, b: $b)
        }
        fragment FragC on Type {
          field2(c: $c)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 4);
    assert!(messages.contains(&&"Variable \"$c\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$a\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$b\" is not defined by operation \"Bar\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$c\" is not defined by operation \"Bar\".".to_owned()));
}
