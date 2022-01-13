use std::collections::HashMap;

use crate::{
    static_graphql::{
        query::{Value, VariableDefinition},
        schema::{self, Type, TypeDefinition},
    },
    validation::utils::find_object_type_by_name,
};

use super::{find_schema_definition, CompositeType, TypeDefinitionExtension};

#[derive(Debug)]
pub struct TypeInfoRegistry<'a> {
    pub query_type: &'a schema::ObjectType,
    pub mutation_type: Option<&'a schema::ObjectType>,
    pub subscription_type: Option<&'a schema::ObjectType>,
    pub type_by_name: HashMap<String, &'a schema::TypeDefinition>,
    pub directives: HashMap<String, &'a schema::DirectiveDefinition>,
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
                    .mutation
                    .clone()
                    .unwrap_or("Mutation".to_string()),
                None => "Mutation".to_string(),
            },
        );
        let subscription_type = find_object_type_by_name(
            &schema,
            match schema_definition {
                Some(schema_definition) => schema_definition
                    .subscription
                    .clone()
                    .unwrap_or("Subscription".to_string()),
                None => "Subscription".to_string(),
            },
        );

        let type_by_name =
            HashMap::from_iter(schema.definitions.iter().filter_map(
                |definition| match definition {
                    schema::Definition::TypeDefinition(type_definition) => {
                        Some((type_definition.name(), type_definition))
                    }
                    _ => None,
                },
            ));

        let directives =
            HashMap::from_iter(schema.definitions.iter().filter_map(
                |definition| match definition {
                    schema::Definition::DirectiveDefinition(directive_definition) => {
                        Some((directive_definition.name.clone(), directive_definition))
                    }
                    _ => None,
                },
            ));

        return TypeInfoRegistry {
            mutation_type,
            query_type,
            subscription_type,
            type_by_name,
            directives,
        };
    }
}

/// This struct is used to mark a "node" or nothing (null, undefined). While tracking TypeInfo, we need to check if there was a node before or not.
#[derive(Debug, Clone, Copy)]
pub enum TypeInfoElementRef<T> {
    Empty,
    Ref(T),
}

pub struct TypeInfo {
    pub type_stack: Vec<TypeInfoElementRef<schema::Type>>,
    pub parent_type_stack: Vec<TypeInfoElementRef<CompositeType>>,
    pub field_def_stack: Vec<TypeInfoElementRef<schema::Field>>,
    pub input_type_stack: Vec<TypeInfoElementRef<PossibleInputType>>,
    pub default_value_stack: Vec<TypeInfoElementRef<Option<Value>>>,
    pub argument: Option<TypeInfoElementRef<schema::InputValue>>,
    pub known_variables: Vec<VariableDefinition>,
}

/// Type (named/list/non-null), schema raw type, and default value
#[derive(Debug, Clone)]
pub enum PossibleInputType {
    Scalar(Type, schema::ScalarType, Option<Value>),
    Enum(Type, schema::EnumType, Option<Value>),
    InputObject(Type, schema::InputObjectType, Option<Value>),
}

impl PossibleInputType {
    pub fn get_type(&self) -> &Type {
        match self {
            PossibleInputType::Scalar(t, _, _) => t,
            PossibleInputType::Enum(t, _, _) => t,
            PossibleInputType::InputObject(t, _, _) => t,
        }
    }

    pub fn get_schema_type_definition(&self) -> TypeDefinition {
        match self {
            PossibleInputType::Scalar(_, t, _) => TypeDefinition::Scalar(t.clone()),
            PossibleInputType::Enum(_, t, _) => TypeDefinition::Enum(t.clone()),
            PossibleInputType::InputObject(_, t, _) => TypeDefinition::InputObject(t.clone()),
        }
    }

    pub fn get_default_value(&self) -> &Option<Value> {
        match self {
            PossibleInputType::Scalar(_, _, d) => d,
            PossibleInputType::Enum(_, _, d) => d,
            PossibleInputType::InputObject(_, _, d) => d,
        }
    }
}

impl TypeInfo {
    pub fn new() -> Self {
        return TypeInfo {
            type_stack: Vec::new(),
            parent_type_stack: Vec::new(),
            input_type_stack: Vec::new(),
            field_def_stack: Vec::new(),
            default_value_stack: Vec::new(),
            known_variables: Vec::new(),
            argument: None,
        };
    }

    pub fn get_argument(&self) -> Option<TypeInfoElementRef<schema::InputValue>> {
        self.argument.clone()
    }

    pub fn enter_argument(&mut self, input_value: TypeInfoElementRef<schema::InputValue>) {
        self.argument = Some(input_value);
    }

    pub fn leave_argument(&mut self) {
        self.argument = None;
    }

    pub fn get_known_variables(&self) -> Vec<VariableDefinition> {
        self.known_variables.clone()
    }

    pub fn enter_known_variables(&mut self, known_variables: Vec<VariableDefinition>) {
        self.known_variables = known_variables
    }

    pub fn leave_known_variables(&mut self) {
        self.known_variables = vec![];
    }

    pub fn get_default_value(&self) -> Option<TypeInfoElementRef<Option<Value>>> {
        self.default_value_stack.last().cloned()
    }

    pub fn enter_default_value(&mut self, default_value: TypeInfoElementRef<Option<Value>>) {
        self.default_value_stack.push(default_value);
    }

    pub fn leave_default_value(&mut self) {
        self.default_value_stack.pop();
    }

    pub fn get_type(&self) -> Option<TypeInfoElementRef<schema::Type>> {
        self.type_stack.last().cloned()
    }

    pub fn enter_type(&mut self, object: TypeInfoElementRef<schema::Type>) {
        self.type_stack.push(object);
    }

    pub fn leave_type(&mut self) {
        self.type_stack.pop();
    }

    pub fn get_input_type(&self) -> Option<TypeInfoElementRef<PossibleInputType>> {
        self.input_type_stack.last().cloned()
    }

    pub fn enter_input_type(&mut self, object: TypeInfoElementRef<PossibleInputType>) {
        self.input_type_stack.push(object);
    }

    pub fn leave_input_type(&mut self) {
        self.input_type_stack.pop();
    }

    pub fn get_parent_type(&self) -> Option<TypeInfoElementRef<CompositeType>> {
        self.parent_type_stack.last().cloned()
    }

    pub fn enter_parent_type(&mut self, object: TypeInfoElementRef<CompositeType>) {
        self.parent_type_stack.push(object);
    }

    pub fn leave_parent_type(&mut self) {
        self.parent_type_stack.pop();
    }

    pub fn get_field_def(&self) -> Option<TypeInfoElementRef<schema::Field>> {
        self.field_def_stack.last().cloned()
    }

    pub fn enter_field_def(&mut self, field: TypeInfoElementRef<schema::Field>) {
        self.field_def_stack.push(field);
    }

    pub fn leave_field_def(&mut self) {
        self.field_def_stack.pop();
    }
}
