
use super::{ValidationRule};
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationError;
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

pub struct LoneAnonymousOperation {
}

impl QueryVisitor<ValidationContext> for LoneAnonymousOperation {
  fn enter_document(&self, _node: &Document, visitor_context: &mut ValidationContext) {
    let operations_count = _node.definitions.iter().filter(|n| {
      match n {
        Definition::Operation(OperationDefinition::SelectionSet(_)) => true,
        Definition::Operation(OperationDefinition::Query(_)) => true,
        Definition::Operation(OperationDefinition::Mutation(_)) => true,
        Definition::Operation(OperationDefinition::Subscription(_)) => true,
        _ => false,
      }
    }).count();

    println!("operations_count: {}, _node: {:?}", operations_count, _node);

    for definition in &_node.definitions {
      match definition {
        Definition::Operation(OperationDefinition::SelectionSet(_)) => {
          if operations_count > 1 {
            visitor_context.report_error(ValidationError{
              message: "This anonymous operation must be the only defined operation.".to_string(),
              locations: vec![], 
            })
          }
        },
        Definition::Operation(OperationDefinition::Query(query)) => {
          if query.name == None && operations_count > 1 {
            visitor_context.report_error(ValidationError{
              message: "This anonymous operation must be the only defined operation.".to_string(),
              locations: vec![query.position.clone()], 
            })
          }
        },
        Definition::Operation(OperationDefinition::Mutation(mutation)) => {
          if mutation.name == None && operations_count > 1 {
            visitor_context.report_error(ValidationError{
              message: "This anonymous operation must be the only defined operation.".to_string(),
              locations: vec![mutation.position.clone()], 
            })
          }
        },
        Definition::Operation(OperationDefinition::Subscription(subscription)) => {
          if subscription.name == None && operations_count > 1 {
            visitor_context.report_error(ValidationError{
              message: "This anonymous operation must be the only defined operation.".to_string(),
              locations: vec![subscription.position.clone()], 
            })
          }
        },
        _ => {},
      };
    }
  }
}

impl ValidationRule for LoneAnonymousOperation {
  fn validate(&self, ctx: &mut ValidationContext) {
    self.visit_document(&ctx.operation.clone(), ctx)
  }
}

#[test]
fn no_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LoneAnonymousOperation {}));
    let errors = test_operation_without_schema(
        "fragment fragA on Type {
          field
        }"
        .to_owned(),
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn one_anon_operation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LoneAnonymousOperation {}));
    let errors = test_operation_without_schema(
        "{
          field
        }"
        .to_owned(),
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn mutiple_named() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LoneAnonymousOperation {}));
    let errors = test_operation_without_schema(
        "query Foo {
          field
        }
        query Bar {
          field
        }"
        .to_owned(),
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn anon_operation_with_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LoneAnonymousOperation {}));
    let errors = test_operation_without_schema(
        "{
          ...Foo
        }
        fragment Foo on Type {
          field
        }"
        .to_owned(),
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn multiple_anon_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LoneAnonymousOperation {}));
    let errors = test_operation_without_schema(
        "{
          fieldA
        }
        {
          fieldB
        }"
        .to_owned(),
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages, vec!["This anonymous operation must be the only defined operation.", "This anonymous operation must be the only defined operation."]);
}

#[test]
fn anon_operation_with_mutation() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LoneAnonymousOperation {}));
    let errors = test_operation_without_schema(
        "{
          fieldA
        }
        mutation Foo {
          fieldB
        }"
        .to_owned(),
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["This anonymous operation must be the only defined operation."]);
}

#[test]
fn anon_operation_with_subscription() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(LoneAnonymousOperation {}));
    let errors = test_operation_without_schema(
        "{
          fieldA
        }
        subscription Foo {
          fieldB
        }"
        .to_owned(),
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["This anonymous operation must be the only defined operation."]);
}

