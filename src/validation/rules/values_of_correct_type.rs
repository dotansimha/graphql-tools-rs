use crate::ast::ext::TypeDefinitionExtension;
use crate::ast::TypeExtension;
use crate::static_graphql::query::{Type, Value};
use crate::static_graphql::schema::{self, TypeDefinition};
use crate::validation::utils::ValidationError;
use crate::{
    ast::{
        visit_document, FieldByNameExtension, OperationVisitor, OperationVisitorContext,
        SchemaDocumentExtension,
    },
    static_graphql::{query, schema::InputValue},
    validation::utils::ValidationErrorContext,
};

use super::ValidationRule;

pub struct ValuesOfCorrectType {
    current_args: Option<Vec<InputValue>>,
}

impl ValuesOfCorrectType {
    pub fn new() -> Self {
        Self { current_args: None }
    }

    pub fn is_valid_literal_value(
        &self,
        schema: &schema::Document,
        type_def: &Option<TypeDefinition>,
        arg_type: &Type,
        arg_value: &Value,
    ) -> bool {
        match arg_type {
            Type::NonNullType(ref inner) => {
                if let Value::Null = arg_value {
                    return false;
                } else {
                    return self.is_valid_literal_value(schema, type_def, inner, arg_value);
                }
            }
            Type::ListType(ref inner) => match *arg_value {
                Value::List(ref items) => items
                    .iter()
                    .all(|i| self.is_valid_literal_value(schema, type_def, inner, &i)),
                ref v => self.is_valid_literal_value(schema, type_def, inner, v),
            },
            Type::NamedType(t) => {
                match (arg_value, type_def) {
                    (Value::Int(_), Some(TypeDefinition::Enum(_))) => return false,
                    (Value::Boolean(_), Some(TypeDefinition::Enum(_))) => return false,
                    (Value::String(_), Some(TypeDefinition::Enum(_))) => return false,
                    (Value::Float(_), Some(TypeDefinition::Enum(_))) => return false,
                    (_, _) => {}
                };

                match *arg_value {
                    Value::Null | Value::Variable(_) => true,
                    Value::Boolean(_)
                    | Value::Float(_)
                    | Value::Int(_)
                    | Value::String(_)
                    | Value::Enum(_) => {
                        return false;
                        /*
                        if let Some(parse_fn) = t.input_value_parse_fn() {
                              parse_fn(v).is_ok()
                          } else {
                              false
                          }
                           */
                    }
                    Value::List(_) => false,
                    Value::Object(ref obj) => {
                        false
                        /*
                         if let MetaType::InputObject(InputObjectMeta {
                               ref input_fields, ..
                           }) = *t
                           {
                               let mut remaining_required_fields = input_fields
                                   .iter()
                                   .filter_map(|f| {
                                       if f.arg_type.is_non_null() {
                                           Some(&f.name)
                                       } else {
                                           None
                                       }
                                   })
                                   .collect::<HashSet<_>>();

                               let all_types_ok = obj.iter().all(|&(ref key, ref value)| {
                                   remaining_required_fields.remove(&key.item);
                                   if let Some(ref arg_type) = input_fields
                                       .iter()
                                       .filter(|f| f.name == key.item)
                                       .map(|f| schema.make_type(&f.arg_type))
                                       .next()
                                   {
                                       is_valid_literal_value(schema, arg_type, &value.item)
                                   } else {
                                       false
                                   }
                               });

                               all_types_ok && remaining_required_fields.is_empty()
                           } else {
                               false
                           }
                        */
                    }
                }
            }
        }
    }
}

impl<'a> OperationVisitor<'a, ValidationErrorContext> for ValuesOfCorrectType {
    fn enter_directive(
        &mut self,
        visitor_context: &mut OperationVisitorContext<'a>,
        _: &mut ValidationErrorContext,
        directive: &query::Directive,
    ) {
        self.current_args = visitor_context
            .directives
            .get(&directive.name)
            .map(|directive_definition| directive_definition.arguments.clone());
    }

    fn leave_directive(
        &mut self,
        _: &mut OperationVisitorContext<'a>,
        _: &mut ValidationErrorContext,
        _: &query::Directive,
    ) {
        self.current_args = None;
    }

    fn enter_field(
        &mut self,
        visitor_context: &mut OperationVisitorContext<'a>,
        _: &mut ValidationErrorContext,
        field: &query::Field,
    ) {
        self.current_args = visitor_context
            .current_parent_type()
            .and_then(|parent_type| visitor_context.schema.type_by_name(&parent_type.name()))
            .and_then(|t| t.field_by_name(&field.name))
            .map(|field_def| field_def.arguments.clone());
    }

    fn leave_field(
        &mut self,
        _: &mut OperationVisitorContext<'a>,
        _: &mut ValidationErrorContext,
        _: &query::Field,
    ) {
        self.current_args = None;
    }

    fn enter_argument(
        &mut self,
        visitor_context: &mut OperationVisitorContext<'a>,
        user_context: &mut ValidationErrorContext,
        (arg_name, arg_value): &(String, query::Value),
    ) {
        if let Some(argument) = self
            .current_args
            .as_ref()
            .and_then(|args| args.iter().find(|a| a.name.eq(arg_name)))
        {
            let schema_type = visitor_context
                .schema
                .type_by_name(&argument.value_type.inner_type());

            if !self.is_valid_literal_value(
                visitor_context.schema,
                &schema_type,
                &argument.value_type,
                &arg_value,
            ) {
                user_context.report_error(ValidationError {
                    message: format!(
                        "Invalid value for argument \"{}\", expected type \"{}\"",
                        arg_name, argument.value_type
                    ),
                    locations: vec![],
                });
            }
        }
    }
}

impl ValidationRule for ValuesOfCorrectType {
    fn validate<'a>(
        &self,
        ctx: &'a mut OperationVisitorContext,
        error_collector: &mut ValidationErrorContext,
    ) {
        visit_document(
            &mut ValuesOfCorrectType::new(),
            &ctx.operation,
            ctx,
            error_collector,
        );
    }
}

#[test]
fn valid_int_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: 2)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_negative_int_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: -2)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_boolean_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            booleanArgField(booleanArg: true)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_string_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringArgField(stringArg: \"foo\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_float_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            floatArgField(floatArg: 1.1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_negative_float_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            floatArgField(floatArg: -1.1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_int_into_float_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            floatArgField(floatArg: 1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_int_into_id_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            idArgField(idArg: 1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_string_into_id_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            idArgField(idArg: \"someIdString\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_enum_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            doesKnowCommand(dogCommand: SIT)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn enum_undefined_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            enumArgField(enumArg: UNKNOWN)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn enum_null_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            enumArgField(enumArg: NO_FUR)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_null_into_nullable() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: null)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);

    let errors = test_operation_with_schema(
        "
        {
          dog(a: null, b: null, c:{ requiredField: true, intField: null }) {
            name
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn invalid_int_into_string() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringArgField(stringArg: 1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: 1"]
    );
}

#[test]
fn invalid_float_into_string() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringArgField(stringArg: 1.0)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: 1.0"]
    );
}

#[test]
fn invalid_bool_into_string() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringArgField(stringArg: true)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: true"]
    );
}

#[test]
fn unquoted_string_to_string() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringArgField(stringArg: BAR)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: BAR"]
    );
}

#[test]
fn invalid_string_into_int() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: \"3\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Int cannot represent non-integer value: \"3\""]
    );
}

#[test]
fn bigint_into_int() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: 829384293849283498239482938)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Int cannot represent non 32-bit signed integer value: 829384293849283498239482938"]
    );
}

#[test]
fn unquoted_string_into_int() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: FOO)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Int cannot represent non-integer value: FOO"]
    );
}

#[test]
fn simple_float_into_int() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: 3.0)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Int cannot represent non-integer value: FOO"]
    );
}

#[test]
fn float_into_int() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            intArgField(intArg: 3.333)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Int cannot represent non-integer value: 3.333"]
    );
}

#[test]
fn string_into_float() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            floatArgField(floatArg: \"3.333\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Float cannot represent non numeric value: \"3.333\""]
    );
}

#[test]
fn boolean_into_float() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            floatArgField(floatArg: true)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Float cannot represent non numeric value: true"]
    );
}

#[test]
fn unquoted_into_float() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            floatArgField(floatArg: FOO)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Float cannot represent non numeric value: FOO"]
    );
}

#[test]
fn int_into_boolean() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            booleanArgField(booleanArg: 2)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Boolean cannot represent non numeric value: 2"]
    );
}

#[test]
fn float_into_boolean() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            booleanArgField(booleanArg: 2.0)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Boolean cannot represent non numeric value: 2.0"]
    );
}

#[test]
fn string_into_boolean() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            booleanArgField(booleanArg: \"true\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Boolean cannot represent non numeric value: true"]
    );
}

#[test]
fn unquoted_into_boolean() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            booleanArgField(booleanArg: TRUE)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Boolean cannot represent non numeric value: TRUE"]
    );
}

#[test]
fn float_into_id() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            idArgField(idArg: 1.0)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["ID cannot represent a non-string and non-integer value: 1.0"]
    );
}

#[test]
fn bool_into_id() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            idArgField(idArg: true)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["ID cannot represent a non-string and non-integer value: true"]
    );
}

#[test]
fn unquoted_into_id() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            idArgField(idArg: SOMETHING)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["ID cannot represent a non-string and non-integer value: SOMETHING"]
    );
}

#[test]
fn int_into_enum() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            doesKnowCommand(dogCommand: 2)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Enum \"DogCommand\" cannot represent non-enum value: 2"]
    );
}

#[test]
fn float_into_enum() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            doesKnowCommand(dogCommand: 1.0)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Enum \"DogCommand\" cannot represent non-enum value: 1.0"]
    );
}

#[test]
fn string_into_enum() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            doesKnowCommand(dogCommand: \"SIT\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Enum \"DogCommand\" cannot represent non-enum value: SIT"]
    );
}

#[test]
fn boolean_into_enum() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            doesKnowCommand(dogCommand: true)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Enum \"DogCommand\" cannot represent non-enum value: true"]
    );
}

#[test]
fn unknown_enum_value_into_enum() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            doesKnowCommand(dogCommand: JUGGLE)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Value \"JUGGLE\" does not exist in \"DogCommand\" enum."]
    );
}

#[test]
fn different_case_enum_value_into_enum() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            doesKnowCommand(dogCommand: sit)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
    assert_eq!(
        messages,
        vec!["Value \"sit\" does not exist in \"DogCommand\" enum."]
    );
}

#[test]
fn valid_list_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringListArgField(stringListArg: [\"one\", null, \"two\"])
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_empty_list_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringListArgField(stringListArg: [])
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_null_list_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringListArgField(stringListArg: null)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn valid_single_value_into_list_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringListArgField(stringListArg: \"one\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn incorrect_item_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringListArgField(stringListArg: [\"one\", 2])
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: 2"]
    );
}

#[test]
fn single_value_of_incorrect_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            stringListArgField(stringListArg: 1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: 1"]
    );
}

#[test]
fn arg_on_optional_arg() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            isHouseTrained(atOtherHomes: true)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn no_arg_on_optional_arg() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog {
            isHouseTrained
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_valid_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleReqs(req1: 1, req2: 2)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_valid_args_reverse_oreder() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleReqs(req2: 2, req1: 1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn no_args_multiple_optional() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleOpts
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn one_arg_multiple_optinals() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleOpts(opt1: 1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn second_arg_multiple_optinals() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleOpts(opt2: 1)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_required_args_on_mixed_list() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleOptAndReq(req1: 3, req2: 4)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn multiple_required_args_and_one_optional_on_mixed_list() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleOptAndReq(req1: 3, req2: 4, opt1: 5)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn all_required_and_one_optional() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleOptAndReq(req1: 3, req2: 4, opt1: 5, opt2: 6)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn incorrect_value_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleReqs(req2: \"two\", req1: \"one\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Int cannot represent non-integer value: \"two\"",
            "Int cannot represent non-integer value: \"one\""
        ]
    )
}

#[test]
fn incorrect_value_and_missing_argument() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleReqs(req1: \"one\")
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Int cannot represent non-integer value: \"one\""]
    );
}

#[test]
fn null_value() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            multipleReqs(req1: null)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Expected value of type \"Int!\", found null."]
    );
}

#[test]
fn optional_arg_despite_required_field_in_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn partial_object_only_required() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: { requiredField: true })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn partial_object_required_field_can_be_falsy() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: { requiredField: false })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn partial_object_including_required() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: { requiredField: true, intField: 4 })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn full_object() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: {
              requiredField: true,
              intField: 4,
              stringField: \"foo\",
              booleanField: false,
              stringListField: [\"one\", \"two\"]
            })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn full_object_with_fields_in_different_order() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: {
              stringListField: [\"one\", \"two\"],
              booleanField: false,
              requiredField: true,
              stringField: \"foo\",
              intField: 4,
            })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn partial_object_missing_required() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: { intField: 4 })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec![
            "Field \"ComplexInput.requiredField\" of required type \"Boolean!\" was not provided."
        ]
    )
}

#[test]
fn partial_object_invalid_field_type() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: {
              stringListField: [\"one\", 2],
              requiredField: true,
            })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: 2"]
    )
}

#[test]
fn partial_object_null_to_non_null_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: {
              requiredField: true,
              nonNullField: null,
            })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Expected value of type \"Boolean!\", found null."]
    )
}

#[test]
fn partial_object_unknown_field_arg() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          complicatedArgs {
            complexArgField(complexArg: {
              requiredField: true,
              invalidField: \"value\"
            })
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["Field \"invalidField\" is not defined by type \"ComplexInput\"."]
    )
}

#[test]
fn allows_custom_scalar_to_accept_complex_literals() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          test1: anyArg(arg: 123)
          test2: anyArg(arg: \"abc\")
          test3: anyArg(arg: [123, \"abc\"])
          test4: anyArg(arg: {deep: [123, \"abc\"]})
        }",
        "
        scalar Any

        type Query {
          anyArg(arg: Any): String
        }
        ",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn with_directives_of_valid_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog @include(if: true) {
            name
          }
          human @skip(if: false) {
            name
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn with_directives_of_invalid_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        {
          dog @include(if: \"yes\") {
            name @skip(if: ENUM)
          }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec![
            "Boolean cannot represent a non boolean value: \"yes\"",
            "Boolean cannot represent a non boolean value: ENUM"
        ]
    )
}

#[test]
fn variables_with_valid_default_values() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        query WithDefaultValues(
          $a: Int = 1,
          $b: String = \"ok\",
          $c: ComplexInput = { requiredField: true, intField: 3 }
          $d: Int! = 123
        ) {
          dog { name }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variables_with_valid_default_null_values() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        query WithDefaultValues(
          $a: Int = null,
          $b: String = null,
          $c: ComplexInput = { requiredField: true, intField: null }
        ) {
          dog { name }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn variables_with_invalid_default_null_values() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        query WithDefaultValues(
          $a: Int! = null,
          $b: String! = null,
          $c: ComplexInput = { requiredField: null, intField: null }
        ) {
          dog { name }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages,
        vec![
            "Expected value of type \"Int!\", found null.",
            "Expected value of type \"String!\", found null.",
            "Expected value of type \"Boolean!\", found null."
        ]
    );
}

#[test]
fn variables_with_invalid_default_values() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        query InvalidDefaultValues(
          $a: Int = \"one\",
          $b: String = 4,
          $c: ComplexInput = \"NotVeryComplex\"
        ) {
          dog { name }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages,
        vec![
            "Int cannot represent non-integer value: \"one\"",
            "String cannot represent a non string value: 4",
            "Expected value of type \"ComplexInput\", found \"NotVeryComplex\"."
        ]
    );
}

#[test]
fn variables_with_complex_invalid_default_values() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        query WithDefaultValues(
          $a: ComplexInput = { requiredField: 123, intField: \"abc\" }
        ) {
          dog { name }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Boolean cannot represent a non boolean value: 123",
            "Int cannot represent non-integer value: \"abc\"",
        ]
    );
}

#[test]
fn complex_variables_missing_required_field() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        query MissingRequiredField($a: ComplexInput = {intField: 3}) {
          dog { name }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec![
            "Field \"ComplexInput.requiredField\" of required type \"Boolean!\" was not provided.",
        ]
    );
}

#[test]
fn list_variables_with_invalid_item() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(ValuesOfCorrectType::new()));
    let errors = test_operation_with_schema(
        "
        query InvalidItem($a: [String] = [\"one\", 2]) {
          dog { name }
        }",
        &TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages,
        vec!["String cannot represent a non string value: 2",]
    );
}
