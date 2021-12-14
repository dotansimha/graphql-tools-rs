use graphql_parser::Pos;
use graphql_parser::{query, schema};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug)]
pub struct ValidationContext {
    pub operation: query::Document<'static, String>,
    pub schema: schema::Document<'static, String>,
    pub fragments: HashMap<String, crate::static_graphql::query::FragmentDefinition>,
    pub validation_errors: Vec<ValidationError>,
}

impl ValidationContext {
    pub fn report_error(&mut self, error: ValidationError) {
        self.validation_errors.push(error);
    }

    pub fn find_schema_definition_by_name(
        &mut self,
        name: String,
    ) -> Option<&schema::TypeDefinition<'static, String>> {
        println!("looking for {} in schema: {}", name, self.schema);

        for definition in &self.schema.definitions {
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
}

#[derive(Debug)]
pub struct ValidationError {
    pub locations: Vec<Pos>,
    pub message: String,
}
