use std::collections::HashMap;

use super::ValidationRule;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{
  ast::SchemaVisitor,
  static_graphql::schema::{Field, ObjectType},
  validation::utils::ValidationContext,
};

/// Unique input field names
///
/// A GraphQL input object value is only valid if all supplied fields are
/// uniquely named.
///
/// See https://spec.graphql.org/draft/#sec-Input-Object-Field-Uniqueness
pub struct UniqueInputFieldNames;

impl<'a> SchemaVisitor<UniqueInputFieldNamesHelper<'a>> for UniqueInputFieldNames {
  fn enter_object_type(
    &self,
    _node: &ObjectType,
    _visitor_context: &mut UniqueInputFieldNamesHelper,
  ) {
    let known = HashMap::new();
    _visitor_context.known_names_vector.push(known);
  }

  fn leave_object_type(
    &self,
    _node: &ObjectType,
    _visitor_context: &mut UniqueInputFieldNamesHelper,
  ) {
    _visitor_context.known_names_vector.pop();
  }

  fn enter_object_type_field(
    &self,
    _node: &Field,
    _type_: &ObjectType,
    _visitor_context: &mut UniqueInputFieldNamesHelper,
  ) {
    let field_name = _node.name.clone();
    let known = _visitor_context.known_names_vector[0].clone();
    let known_field = known.get(&field_name);
    match known_field {
      None => {
        let mut known = known.clone();
        known.insert(field_name, _type_.clone());
        _visitor_context.known_names_vector[0] = known;
      }
      Some(known_field) => {
        _visitor_context
          .errors_context
          .errors
          .push(ValidationError {
            locations: vec![known_field.position.clone()],
            message: format!(
              "There can be only one input field named  \"{}\".",
              known_field.name.clone()
            ),
          });
      }
    }
  }
}

struct UniqueInputFieldNamesHelper<'a> {
  known_names_vector: Vec<HashMap<String, ObjectType>>,
  validation_context: &'a ValidationContext<'a>,
  errors_context: ValidationErrorContext<'a>,
}

impl<'a> UniqueInputFieldNamesHelper<'a> {
  fn new(validation_context: &'a ValidationContext<'a>) -> Self {
    let known_names = HashMap::new();
    let mut known_names_vector = Vec::new();
    known_names_vector.push(known_names);
    Self {
      known_names_vector: known_names_vector,
      validation_context: validation_context,
      errors_context: ValidationErrorContext::new(validation_context),
    }
  }
}

impl ValidationRule for UniqueInputFieldNames {
  fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
    let mut helper = UniqueInputFieldNamesHelper::new(ctx.clone());
    self.visit_schema_document(&ctx.schema.clone(), &mut helper);
    helper.errors_context.errors
  }
}

#[test]
fn no_fragments() {
  use crate::validation::test_utils::*;

  let mut plan = create_plan_from_rule(Box::new(UniqueInputFieldNames {}));
  let errors = test_operation_without_schema(
    " {
      field(arg: { f: true, f:false })
    }",
    &mut plan,
  );

  assert_eq!(get_messages(&errors).len(), 0);
}
