use crate::static_graphql::{query, schema};
use graphql_parser::Pos;
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug)]
pub struct ValidationContext {
    pub operation: query::Document,
    pub schema: schema::Document,
    pub fragments: HashMap<String, query::FragmentDefinition>,
    pub validation_errors: Vec<ValidationError>,
}

impl ValidationContext {
    /// Reports a GraphQL validatioמ error on specified locations
    pub fn report_error(&mut self, error: ValidationError) {
        self.validation_errors.push(error);
    }

    /// Utilities for getting a specific source schema definition by it's name
    pub fn find_schema_definition_by_name(
        &mut self,
        name: String,
    ) -> Option<&schema::TypeDefinition> {
        find_schema_definition_by_name(&self.schema, name)
    }
}

pub fn find_schema_definition_by_name(
    schema: &schema::Document,
    name: String,
) -> Option<&schema::TypeDefinition> {
    for definition in &schema.definitions {
        match definition {
            schema::Definition::TypeDefinition(type_definition) => match type_definition {
                schema::TypeDefinition::Object(object) if object.name.eq(&name) => {
                    return Some(type_definition)
                }
                schema::TypeDefinition::Scalar(object) if object.name.eq(&name) => {
                    return Some(type_definition)
                }
                schema::TypeDefinition::Interface(object) if object.name.eq(&name) => {
                    return Some(type_definition)
                }
                schema::TypeDefinition::InputObject(object) if object.name.eq(&name) => {
                    return Some(type_definition)
                }
                schema::TypeDefinition::Enum(object) if object.name.eq(&name) => {
                    return Some(type_definition)
                }
                schema::TypeDefinition::Union(object) if object.name.eq(&name) => {
                    return Some(type_definition)
                }
                _ => {}
            },
            _ => {}
        }
    }

    None
}

pub fn find_object_type_by_name(
    schema: &schema::Document,
    name: String,
) -> Option<&schema::ObjectType> {
    for definition in &schema.definitions {
        match definition {
            schema::Definition::TypeDefinition(type_definition) => match type_definition {
                schema::TypeDefinition::Object(object) if object.name.eq(&name) => {
                    return Some(object)
                }
                _ => {}
            },
            _ => {}
        }
    }

    None
}

#[derive(Debug)]
pub struct ValidationError {
    pub locations: Vec<Pos>,
    pub message: String,
}
