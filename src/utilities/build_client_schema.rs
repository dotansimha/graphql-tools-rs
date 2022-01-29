use std::collections::HashMap;

use crate::introspection::{
    IntrospectionEnumType, IntrospectionField, IntrospectionInputValue, IntrospectionObjectType,
    IntrospectionOutputTypeRef, IntrospectionSchema, IntrospectionType,
};
use graphql_parser::{schema, Pos};

fn build_output_type<'a>(src: &'a IntrospectionOutputTypeRef) -> schema::Type<'a, String> {
    match src {
        IntrospectionOutputTypeRef::ENUM(name_ref) => schema::Type::NamedType(name_ref.name),
        IntrospectionOutputTypeRef::INPUT_OBJECT(name_ref) => {
            schema::Type::NamedType(name_ref.name)
        }
        IntrospectionOutputTypeRef::INTERFACE(name_ref) => schema::Type::NamedType(name_ref.name),
        IntrospectionOutputTypeRef::UNION(name_ref) => schema::Type::NamedType(name_ref.name),
        IntrospectionOutputTypeRef::SCALAR(name_ref) => schema::Type::NamedType(name_ref.name),
        IntrospectionOutputTypeRef::OBJECT(name_ref) => schema::Type::NamedType(name_ref.name),
        IntrospectionOutputTypeRef::LIST { of_type } => {
            let inner = build_output_type(of_type.expect("missing inner type").as_ref());

            schema::Type::ListType(Box::new(inner))
        }
        IntrospectionOutputTypeRef::NON_NULL { of_type } => {
            let inner = build_output_type(of_type.expect("missing inner type").as_ref());
            schema::Type::NonNullType(Box::new(inner))
        }
    }
}

fn build_value_from_string<'a>(src: &'a String) -> schema::Value<'a, String> {}

fn build_args<'a>(src: &'a Vec<IntrospectionInputValue>) -> Vec<schema::InputValue<'a, String>> {
    src.iter()
        .map(|input_value| schema::InputValue {
            position: Pos { column: 0, line: 0 },
            name: input_value.name,
            description: input_value.description,
            default_value: input_value
                .default_value
                .map(|v| build_value_from_string(&v)),
        })
        .collect()
}

fn build_fields<'a>(fields_arr: Vec<IntrospectionField>) -> Vec<schema::Field<'a, String>> {
    fields_arr
        .iter()
        .map(|field| schema::Field {
            position: Pos { column: 0, line: 0 },
            name: field.name,
            description: field.description,
            field_type: build_output_type(&field.type_ref),
            arguments: build_args(&field.args),
            directives: vec![],
        })
        .collect()
}

fn build_object<'a>(src: &'a IntrospectionObjectType) -> schema::ObjectType<'a, String> {
    schema::ObjectType {
        position: Pos { column: 0, line: 0 },
        description: src.description,
        name: src.name,
        directives: vec![],
        implements_interfaces: src.interfaces.iter().map(|i| i.name).collect(),
        fields: build_fields(src.fields),
    }
}

fn build_enum<'a>(src: &'a IntrospectionEnumType) -> schema::EnumType<'a, String> {
    schema::EnumType {
        position: Pos { column: 0, line: 0 },
        description: src.description,
        name: src.name,
        directives: vec![],
        values: src
            .enum_values
            .iter()
            .map(|enum_value| schema::EnumValue {
                position: Pos { column: 0, line: 0 },
                description: enum_value.description,
                name: enum_value.name,
                directives: vec![],
            })
            .collect(),
    }
}

fn build_type<'a>(src: &'a IntrospectionType) -> schema::TypeDefinition<'a, String> {
    match &src {
        IntrospectionType::ENUM(e) => schema::TypeDefinition::Enum(build_enum(e)),
        IntrospectionType::OBJECT(o) => schema::TypeDefinition::Object(build_object(o)),
        IntrospectionType::INPUT_OBJECT(io) => {
            schema::TypeDefinition::InputObject(build_input_object(io))
        }
        IntrospectionType::SCALAR(s) => schema::TypeDefinition::Scalar(build_scalar(s)),
        IntrospectionType::UNION(u) => schema::TypeDefinition::Union(build_union(u)),
        IntrospectionType::INTERFACE(i) => schema::TypeDefinition::Interface(build_inferface(i)),
    }
}

pub fn build_client_schema<'a>(introspection: &'a IntrospectionSchema) {
    let type_map = HashMap::from_iter(
        introspection
            .types
            .iter()
            .map(|t| (t.name(), build_type(t))),
    );
}
