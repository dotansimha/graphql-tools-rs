use crate::static_graphql::schema;

pub struct DefaultVisitorContext;

pub fn find_schema_definition(schema: &schema::Document) -> Option<&schema::SchemaDefinition> {
    schema
        .definitions
        .iter()
        .find_map(|definition| match definition {
            schema::Definition::SchemaDefinition(schema_definition) => Some(schema_definition),
            _ => None,
        })
}

pub fn get_named_type(t: &schema::Type) -> String {
    match t {
        schema::Type::NamedType(name) => name.clone(),
        schema::Type::ListType(inner_type) => get_named_type(inner_type),
        schema::Type::NonNullType(inner_type) => get_named_type(inner_type),
    }
}
