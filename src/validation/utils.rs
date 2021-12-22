use crate::{
    ast::TypeInfoRegistry,
    static_graphql::{query, schema},
};
use graphql_parser::Pos;
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug)]
pub struct ValidationContext<'a> {
    pub operation: query::Document,
    pub schema: schema::Document,
    pub fragments: HashMap<String, query::FragmentDefinition>,
    pub type_info_registry: Option<TypeInfoRegistry<'a>>,
}

impl<'a> ValidationContext<'a> {
    /// Utilities for getting a specific source schema definition by it's name
    pub fn find_schema_definition_by_name(
        &mut self,
        name: String,
    ) -> Option<&schema::TypeDefinition> {
        find_schema_definition_by_name(&self.schema, name)
    }
}

pub struct ValidationErrorContext<'a> {
    pub ctx: &'a ValidationContext<'a>,
    pub errors: Vec<ValidationError>,
}

impl<'a> ValidationErrorContext<'a> {
    pub fn new(ctx: &'a ValidationContext<'a>) -> ValidationErrorContext<'a> {
        ValidationErrorContext {
            ctx,
            errors: vec![],
        }
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
