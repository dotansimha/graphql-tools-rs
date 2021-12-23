use crate::static_graphql::schema::{self, Field, InterfaceType, ObjectType, UnionType};

pub trait AstNodeWithFields {
    fn find_field(&self, name: String) -> Option<&Field>;
}

impl AstNodeWithFields for ObjectType {
    fn find_field(&self, name: String) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl AstNodeWithFields for InterfaceType {
    fn find_field(&self, name: String) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl AstNodeWithFields for UnionType {
    fn find_field(&self, _name: String) -> Option<&Field> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum CompositeType {
    Object(schema::ObjectType),
    Interface(schema::InterfaceType),
    Union(schema::UnionType),
}

impl CompositeType {
    pub fn find_field(&self, name: String) -> Option<&Field> {
        match self {
            CompositeType::Object(o) => o.find_field(name),
            CompositeType::Interface(i) => i.find_field(name),
            CompositeType::Union(u) => u.find_field(name),
        }
    }

    pub fn from_type_definition(t: &schema::TypeDefinition) -> Option<Self> {
        match t {
            schema::TypeDefinition::Object(o) => Some(CompositeType::Object(o.clone())),
            schema::TypeDefinition::Interface(i) => Some(CompositeType::Interface(i.clone())),
            schema::TypeDefinition::Union(u) => Some(CompositeType::Union(u.clone())),
            _ => None,
        }
    }
}

pub trait TypeDefinitionExtension {
    fn is_leaf_type(&self) -> bool;
    fn is_composite_type(&self) -> bool;
    fn is_input_type(&self) -> bool;
    fn name(&self) -> String;
}

impl TypeDefinitionExtension for schema::TypeDefinition {
    fn name(&self) -> String {
        match self {
            schema::TypeDefinition::Object(o) => o.name.clone(),
            schema::TypeDefinition::Interface(i) => i.name.clone(),
            schema::TypeDefinition::Union(u) => u.name.clone(),
            schema::TypeDefinition::Scalar(s) => s.name.clone(),
            schema::TypeDefinition::Enum(e) => e.name.clone(),
            schema::TypeDefinition::InputObject(i) => i.name.clone(),
        }
    }

    fn is_leaf_type(&self) -> bool {
        match self {
            schema::TypeDefinition::Object(o) => false,
            schema::TypeDefinition::Interface(i) => false,
            schema::TypeDefinition::Union(u) => false,
            schema::TypeDefinition::Scalar(u) => true,
            schema::TypeDefinition::Enum(u) => true,
            schema::TypeDefinition::InputObject(u) => false,
        }
    }

    fn is_input_type(&self) -> bool {
        match self {
            schema::TypeDefinition::Object(o) => false,
            schema::TypeDefinition::Interface(i) => false,
            schema::TypeDefinition::Union(u) => false,
            schema::TypeDefinition::Scalar(u) => false,
            schema::TypeDefinition::Enum(u) => false,
            schema::TypeDefinition::InputObject(u) => true,
        }
    }

    fn is_composite_type(&self) -> bool {
        match self {
            schema::TypeDefinition::Object(o) => true,
            schema::TypeDefinition::Interface(i) => true,
            schema::TypeDefinition::Union(u) => true,
            schema::TypeDefinition::Scalar(u) => false,
            schema::TypeDefinition::Enum(u) => false,
            schema::TypeDefinition::InputObject(u) => false,
        }
    }
}
