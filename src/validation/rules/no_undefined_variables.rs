use super::ValidationRule;
use crate::ast::{
    visit_document, AstNodeWithName, OperationVisitor, OperationVisitorContext, ValueExtension,
};
use crate::static_graphql::query::{self, OperationDefinition};
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use std::collections::{HashMap, HashSet};

/// No undefined variables
///
/// A GraphQL operation is only valid if all variables encountered, both directly
/// and via fragment spreads, are defined by that operation.
///
/// See https://spec.graphql.org/draft/#sec-All-Variable-Uses-Defined
pub struct NoUndefinedVariables {
    current_scope: Option<Scope>,
    defined_variables: HashMap<Option<String>, HashSet<String>>,
    used_variables: HashMap<Scope, Vec<String>>,
    spreads: HashMap<Scope, Vec<String>>,
}

impl NoUndefinedVariables {
    pub fn new() -> Self {
        Self {
            current_scope: None,
            defined_variables: HashMap::new(),
            used_variables: HashMap::new(),
            spreads: HashMap::new(),
        }
    }
}

impl NoUndefinedVariables {
    fn find_undefined_vars(
        &self,
        from: &Scope,
        defined: &HashSet<String>,
        unused: &mut HashSet<String>,
        visited: &mut HashSet<Scope>,
    ) {
        if visited.contains(from) {
            return;
        }

        visited.insert(from.clone());

        if let Some(used_vars) = self.used_variables.get(from) {
            for var in used_vars {
                if !defined.contains(var) {
                    unused.insert(var.clone());
                }
            }
        }

        if let Some(spreads) = self.spreads.get(from) {
            for spread in spreads {
                self.find_undefined_vars(
                    &Scope::Fragment(spread.clone()),
                    defined,
                    unused,
                    visited,
                );
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    Operation(Option<String>),
    Fragment(String),
}

impl<'a> OperationVisitor<'a, ValidationErrorContext> for NoUndefinedVariables {
    fn enter_operation_definition(
        &mut self,
        _: &mut OperationVisitorContext<ValidationErrorContext>,
        operation_definition: &OperationDefinition,
    ) {
        let op_name = operation_definition.node_name();
        self.current_scope = Some(Scope::Operation(op_name.clone()));
        self.defined_variables.insert(op_name, HashSet::new());
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut OperationVisitorContext<ValidationErrorContext>,
        fragment_definition: &query::FragmentDefinition,
    ) {
        self.current_scope = Some(Scope::Fragment(fragment_definition.name.clone()));
    }

    fn enter_fragment_spread(
        &mut self,
        _: &mut OperationVisitorContext<ValidationErrorContext>,
        fragment_spread: &query::FragmentSpread,
    ) {
        if let Some(scope) = &self.current_scope {
            self.spreads
                .entry(scope.clone())
                .or_insert_with(Vec::new)
                .push(fragment_spread.fragment_name.clone());
        }
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut OperationVisitorContext<ValidationErrorContext>,
        variable_definition: &query::VariableDefinition,
    ) {
        if let Some(Scope::Operation(ref name)) = self.current_scope {
            if let Some(vars) = self.defined_variables.get_mut(name) {
                vars.insert(variable_definition.name.clone());
            }
        }
    }

    fn enter_argument(
        &mut self,
        _: &mut OperationVisitorContext<ValidationErrorContext>,
        (_arg_name, arg_value): &(String, query::Value),
    ) {
        if let Some(ref scope) = self.current_scope {
            self.used_variables
                .entry(scope.clone())
                .or_insert_with(Vec::new)
                .append(&mut arg_value.variables_in_use());
        }
    }

    fn leave_document(
        &mut self,
        visitor_context: &mut OperationVisitorContext<ValidationErrorContext>,
        _: &query::Document,
    ) {
        for (op_name, def_vars) in &self.defined_variables {
            let mut unused = HashSet::new();
            let mut visited = HashSet::new();

            self.find_undefined_vars(
                &Scope::Operation(op_name.clone()),
                &def_vars,
                &mut unused,
                &mut visited,
            );

            unused.iter().for_each(|var| {
                visitor_context.user_context.report_error(ValidationError {
                    message: error_message(&var, op_name),
                    locations: vec![],
                })
            })
        }
    }
}

fn error_message(var_name: &String, op_name: &Option<String>) -> String {
    if let Some(op_name) = op_name {
        format!(
            r#"Variable "${}" is not defined by operation "{}"."#,
            var_name, op_name
        )
    } else {
        format!(r#"Variable "${}" is not defined."#, var_name)
    }
}

impl ValidationRule for NoUndefinedVariables {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = ValidationErrorContext::new();

        visit_document(
            &mut NoUndefinedVariables::new(),
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut helper, &ctx.operation, &ctx.schema),
        );

        helper.errors
    }
}

#[test]
fn all_variables_defined() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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
    assert!(messages.contains(&&"Variable \"$b\" is not defined by operation \"Bar\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$b\" is not defined by operation \"Foo\".".to_owned()));
}

#[test]
fn variables_in_fragment_not_defined_by_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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
    assert!(messages.contains(&&"Variable \"$a\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$b\" is not defined by operation \"Bar\".".to_owned()));
}

#[test]
fn variable_in_fragment_used_by_other_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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
    assert!(messages.contains(&&"Variable \"$a\" is not defined by operation \"Foo\".".to_owned()));
    assert!(messages.contains(&&"Variable \"$b\" is not defined by operation \"Bar\".".to_owned()));
}
#[test]
fn multiple_undefined_variables_produce_multiple_errors() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUndefinedVariables::new()));
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
