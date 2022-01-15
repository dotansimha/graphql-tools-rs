use super::ValidationRule;
use crate::ast::{visit_document, AstNodeWithName, OperationVisitor, OperationVisitorContext};
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::ext::AstWithVariables, validation::utils::ValidationContext};
use std::collections::HashSet;

/// No undefined variables
///
/// A GraphQL operation is only valid if all variables encountered, both directly
/// and via fragment spreads, are defined by that operation.
///
/// See https://spec.graphql.org/draft/#sec-All-Variable-Uses-Defined
pub struct NoUndefinedVariables;

impl<'a> OperationVisitor<'a, NoUndefinedVariablesHelper> for NoUndefinedVariables {
    fn enter_variable_definition(
        &mut self,
        visitor_context: &mut crate::ast::OperationVisitorContext<NoUndefinedVariablesHelper>,
        variable_definition: &crate::static_graphql::query::VariableDefinition,
    ) {
        visitor_context
            .user_context
            .current_op_variables
            .insert(variable_definition.name.clone());
    }

    fn enter_operation_definition(
        &mut self,
        visitor_context: &mut crate::ast::OperationVisitorContext<NoUndefinedVariablesHelper>,
        _: &crate::static_graphql::query::OperationDefinition,
    ) {
        visitor_context.user_context.current_op_variables.clear();
    }

    fn leave_operation_definition(
        &mut self,
        visitor_context: &mut crate::ast::OperationVisitorContext<NoUndefinedVariablesHelper>,
        operation_definition: &crate::static_graphql::query::OperationDefinition,
    ) {
        let in_use = operation_definition.get_variables_in_use(&visitor_context.known_fragments);

        in_use.iter().for_each(|v| {
            if !visitor_context
                .user_context
                .current_op_variables
                .contains(v)
            {
                visitor_context
                    .user_context
                    .error_ctx
                    .report_error(ValidationError {
                        message: match operation_definition.node_name() {
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

struct NoUndefinedVariablesHelper {
    error_ctx: ValidationErrorContext,
    current_op_variables: HashSet<String>,
}

impl NoUndefinedVariablesHelper {
    fn new() -> Self {
        Self {
            error_ctx: ValidationErrorContext::new(),
            current_op_variables: HashSet::new(),
        }
    }
}

impl ValidationRule for NoUndefinedVariables {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = NoUndefinedVariablesHelper::new();

        visit_document(
            &mut NoUndefinedVariables {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut helper, &ctx.operation, &ctx.schema),
        );

        helper.error_ctx.errors
    }
}

#[test]
fn all_variables_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(a: $a, b: $b, c: $c)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn all_variables_deeply_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(a: $a) {
            field(b: $b) {
              field(c: $c)
            }
          }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn all_variables_deeply_in_inline_fragments_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_with_schema(
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
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn all_variables_in_fragments_deeply_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_within_single_fragment_defined_in_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String) {
          ...FragA
        }
        query Bar($a: String) {
          ...FragA
        }
        fragment FragA on Type {
          field(a: $a)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_within_fragments_defined_in_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_within_recursive_fragment_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
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

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variable_not_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables {}));
    let errors = test_operation_with_schema(
        "query Foo($a: String, $b: String, $c: String) {
          field(a: $a, b: $b, c: $c, d: $d)
        }",
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
        "{
          field(a: $a)
        }",
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
        "query Foo($b: String) {
          field(a: $a, b: $b, c: $c)
        }",
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
        "{
          ...FragA
        }
        fragment FragA on Type {
          field(a: $a)
        }",
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
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
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
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
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
        "query Foo($a: String) {
          ...FragAB
        }
        query Bar($a: String) {
          ...FragAB
        }
        fragment FragAB on Type {
          field(a: $a, b: $b)
        }",
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
        "query Foo($b: String) {
          ...FragAB
        }
        query Bar($a: String) {
          ...FragAB
        }
        fragment FragAB on Type {
          field(a: $a, b: $b)
        }",
        TEST_SCHEMA,
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
    let errors = test_operation_with_schema(
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
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 4);
    assert!(messages.contains(&&"Variable \"$c\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$a\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$b\" is not defined by operation \"Bar\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$c\" is not defined by operation \"Bar\".".to_owned()));
}
