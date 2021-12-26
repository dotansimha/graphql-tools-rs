use std::collections::HashMap;

use crate::{
    static_graphql::schema::{self},
    validation::utils::find_object_type_by_name,
};

use super::{find_schema_definition, CompositeType};

#[derive(Debug)]
pub struct TypeInfoRegistry<'a> {
    pub query_type: &'a schema::ObjectType,
    pub mutation_type: Option<&'a schema::ObjectType>,
    pub subscription_type: Option<&'a schema::ObjectType>,
    pub type_by_name: HashMap<String, &'a schema::TypeDefinition>,
}

impl<'a> TypeInfoRegistry<'a> {
    pub fn new(schema: &'a schema::Document) -> Self {
        let schema_definition = find_schema_definition(&schema);
        let query_type = find_object_type_by_name(
            &schema,
            match schema_definition {
                Some(schema_definition) => schema_definition
                    .query
                    .clone()
                    .unwrap_or("Query".to_string()),
                None => "Query".to_string(),
            },
        )
        .expect("Schema does not contain a Query root type");
        let mutation_type = find_object_type_by_name(
            &schema,
            match schema_definition {
                Some(schema_definition) => schema_definition
                    .query
                    .clone()
                    .unwrap_or("Mutation".to_string()),
                None => "Mutation".to_string(),
            },
        );
        let subscription_type = find_object_type_by_name(
            &schema,
            match schema_definition {
                Some(schema_definition) => schema_definition
                    .query
                    .clone()
                    .unwrap_or("Subscription".to_string()),
                None => "Subscription".to_string(),
            },
        );

        let type_by_name =
            HashMap::from_iter(schema.definitions.iter().filter_map(
                |definition| match definition {
                    schema::Definition::TypeDefinition(type_definition) => match type_definition {
                        schema::TypeDefinition::Object(object) => {
                            Some((object.name.clone(), type_definition))
                        }
                        schema::TypeDefinition::Scalar(object) => {
                            Some((object.name.clone(), type_definition))
                        }
                        schema::TypeDefinition::Interface(object) => {
                            Some((object.name.clone(), type_definition))
                        }
                        schema::TypeDefinition::InputObject(object) => {
                            Some((object.name.clone(), type_definition))
                        }
                        schema::TypeDefinition::Enum(object) => {
                            Some((object.name.clone(), type_definition))
                        }
                        schema::TypeDefinition::Union(object) => {
                            Some((object.name.clone(), type_definition))
                        }
                    },
                    _ => None,
                },
            ));

        return TypeInfoRegistry {
            mutation_type,
            query_type,
            subscription_type,
            type_by_name,
        };
    }
}

pub struct TypeInfo {
    pub type_stack: Vec<schema::Type>,
    pub parent_type_stack: Vec<CompositeType>,
    pub field_def_stack: Vec<schema::Field>,
    pub input_type_stack: Vec<schema::InputObjectType>,
    pub argument: Option<schema::InputValue>,
}

impl TypeInfo {
    pub fn new() -> Self {
        return TypeInfo {
            type_stack: Vec::new(),
            parent_type_stack: Vec::new(),
            input_type_stack: Vec::new(),
            field_def_stack: Vec::new(),
            argument: None,
        };
    }

    pub fn get_argument(&self) -> Option<schema::InputValue> {
        self.argument.clone()
    }

    pub fn enter_argument(&mut self, input_value: schema::InputValue) {
        self.argument = Some(input_value);
    }

    pub fn leave_argument(&mut self) {
        self.argument = None;
    }

    pub fn get_type(&self) -> Option<schema::Type> {
        self.type_stack.last().cloned()
    }

    pub fn enter_type(&mut self, object: schema::Type) {
        self.type_stack.push(object);
    }

    pub fn leave_type(&mut self) {
        self.type_stack.pop();
    }

    pub fn get_input_type(&self) -> Option<schema::InputObjectType> {
        self.input_type_stack.last().cloned()
    }

    pub fn enter_input_type(&mut self, object: schema::InputObjectType) {
        self.input_type_stack.push(object);
    }

    pub fn leave_input_type(&mut self) {
        self.input_type_stack.pop();
    }

    pub fn get_parent_type(&self) -> Option<CompositeType> {
        self.parent_type_stack.last().cloned()
    }

    pub fn enter_parent_type(&mut self, object: CompositeType) {
        self.parent_type_stack.push(object);
    }

    pub fn leave_parent_type(&mut self) {
        self.parent_type_stack.pop();
    }

    pub fn get_field_def(&self) -> Option<schema::Field> {
        self.field_def_stack.last().cloned()
    }

    pub fn enter_field_def(&mut self, field: schema::Field) {
        self.field_def_stack.push(field);
    }

    pub fn leave_field_def(&mut self) {
        self.field_def_stack.pop();
    }
}
