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
    fn find_field(&self, name: String) -> Option<&Field> {
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
