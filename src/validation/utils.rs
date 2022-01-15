use crate::static_graphql::{query, schema};
use graphql_parser::Pos;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ValidationContext<'a> {
    pub operation: &'a query::Document,
    pub schema: &'a schema::Document,
}

impl<'a> ValidationContext<'a> {
    /// Utilities for getting a specific source schema definition by it's name
    pub fn find_schema_definition_by_name(&self, name: String) -> Option<&schema::TypeDefinition> {
        find_schema_definition_by_name(&self.schema, name)
    }
}

pub struct ValidationErrorContext {
    pub errors: Vec<ValidationError>,
}

impl ValidationErrorContext {
    pub fn new() -> ValidationErrorContext {
        ValidationErrorContext { errors: vec![] }
    }

    pub fn report_error(&mut self, error: ValidationError) {
        self.errors.push(error);
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
