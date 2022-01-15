use crate::ast::ext::{
    ImplementingInterfaceExtension, PossibleTypesExtension, TypeDefinitionExtension,
};
use crate::static_graphql::schema::{self, TypeDefinition};

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

/**
 * Extracts nested NamedType from a potentially recursive wrapped definition.
 *
 * Example: Returns String from [String] or String!
 */
pub fn get_named_type(t: &schema::Type) -> String {
    match t {
        schema::Type::NamedType(name) => name.clone(),
        schema::Type::ListType(inner_type) => get_named_type(inner_type),
        schema::Type::NonNullType(inner_type) => get_named_type(inner_type),
    }
}

/**
 * Provided two composite types, determine if they "overlap". Two composite
 * types overlap when the Sets of possible concrete types for each intersect.
 *
 * This is often used to determine if a fragment of a given type could possibly
 * be visited in a context of another type.
 *
 * This function is commutative.
 */
pub fn do_types_overlap(
    schema: &schema::Document,
    t1: &schema::TypeDefinition,
    t2: &schema::TypeDefinition,
) -> bool {
    if t1.name().eq(&t2.name()) {
        return true;
    }

    if t1.is_abstract_type() {
        if t2.is_abstract_type() {
            let possible_types = t1.possible_types(schema);

            return possible_types
                .into_iter()
                .filter(|possible_type| {
                    t2.has_sub_type(&TypeDefinition::Object(possible_type.clone()))
                })
                .count()
                > 0;
        }

        return t1.has_sub_type(t2);
    }

    if t2.is_abstract_type() {
        return t2.has_sub_type(t1);
    }

    false
}
