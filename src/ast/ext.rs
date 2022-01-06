use crate::static_graphql::query::{self, FragmentSpread};
use crate::static_graphql::schema::{
    self, Field, InterfaceType, ObjectType, TypeDefinition, UnionType,
};

use super::{get_named_type, TypeInfoElementRef, TypeInfoRegistry};

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

pub trait AstTypeRef {
    fn named_type(&self) -> String;
}

impl AstTypeRef for query::Type {
    fn named_type(&self) -> String {
        get_named_type(self)
    }
}

#[derive(Debug, Clone)]
pub enum CompositeType {
    Object(schema::ObjectType),
    Interface(schema::InterfaceType),
    Union(schema::UnionType),
}

impl TypeInfoElementRef<CompositeType> {
    pub fn find_field(&self, name: String) -> Option<&Field> {
        match self {
            TypeInfoElementRef::Empty => None,
            TypeInfoElementRef::Ref(composite_type) => composite_type.find_field(name),
        }
    }
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

    pub fn as_type_definition(&self) -> schema::TypeDefinition {
        match self {
            CompositeType::Object(o) => schema::TypeDefinition::Object(o.clone()),
            CompositeType::Interface(o) => schema::TypeDefinition::Interface(o.clone()),
            CompositeType::Union(o) => schema::TypeDefinition::Union(o.clone()),
        }
    }
}

pub trait AbstractTypeDefinitionExtension {
    fn is_implemented_by(&self, other_type: &dyn ImplementingInterfaceExtension) -> bool;
}

pub trait TypeDefinitionExtension {
    fn is_leaf_type(&self) -> bool;
    fn is_composite_type(&self) -> bool;
    fn is_input_type(&self) -> bool;
    fn is_abstract_type(&self) -> bool;
    fn name(&self) -> String;
}

pub trait ImplementingInterfaceExtension {
    fn interfaces(&self) -> Vec<String>;
    fn has_sub_type(&self, other_type: &TypeDefinition) -> bool;
}

impl ImplementingInterfaceExtension for TypeDefinition {
    fn interfaces(&self) -> Vec<String> {
        match self {
            schema::TypeDefinition::Object(o) => o.interfaces(),
            schema::TypeDefinition::Interface(i) => i.interfaces(),
            _ => vec![],
        }
    }

    fn has_sub_type(&self, other_type: &TypeDefinition) -> bool {
        match self {
            TypeDefinition::Interface(interface_type) => {
                return interface_type.is_implemented_by(other_type)
            }
            TypeDefinition::Union(union_type) => {
                return union_type.has_sub_type(&other_type.name())
            }
            _ => return false,
        }
    }
}

pub trait PossibleTypesExtension<'a> {
    fn possible_types(&self, type_info_registry: &TypeInfoRegistry) -> Vec<ObjectType>;
}

impl<'a> PossibleTypesExtension<'a> for TypeDefinition {
    fn possible_types(&self, type_info_registry: &TypeInfoRegistry) -> Vec<ObjectType> {
        match self {
            TypeDefinition::Object(_) => vec![],
            TypeDefinition::InputObject(_) => vec![],
            TypeDefinition::Enum(_) => vec![],
            TypeDefinition::Scalar(_) => vec![],
            TypeDefinition::Interface(i) => type_info_registry
                .type_by_name
                .iter()
                .filter_map(|(_type_name, type_def)| {
                    if let TypeDefinition::Object(o) = type_def {
                        if i.is_implemented_by(*type_def) {
                            return Some(o.clone());
                        }
                    }

                    None
                })
                .collect(),
            TypeDefinition::Union(u) => u
                .types
                .iter()
                .filter_map(|type_name| {
                    if let Some(TypeDefinition::Object(o)) =
                        type_info_registry.type_by_name.get(type_name)
                    {
                        return Some(o.clone());
                    }

                    None
                })
                .collect(),
        }
    }
}

impl ImplementingInterfaceExtension for InterfaceType {
    fn interfaces(&self) -> Vec<String> {
        self.implements_interfaces.clone()
    }

    fn has_sub_type(&self, other_type: &TypeDefinition) -> bool {
        self.is_implemented_by(other_type)
    }
}

impl ImplementingInterfaceExtension for ObjectType {
    fn interfaces(&self) -> Vec<String> {
        self.implements_interfaces.clone()
    }

    fn has_sub_type(&self, _other_type: &TypeDefinition) -> bool {
        false
    }
}

pub trait SubTypeExtension {
    fn has_sub_type(&self, other_type_name: &String) -> bool;
}

impl SubTypeExtension for UnionType {
    fn has_sub_type(&self, other_type_name: &String) -> bool {
        self.types.iter().find(|v| other_type_name.eq(*v)).is_some()
    }
}

impl AbstractTypeDefinitionExtension for InterfaceType {
    fn is_implemented_by(&self, other_type: &dyn ImplementingInterfaceExtension) -> bool {
        other_type
            .interfaces()
            .iter()
            .find(|v| self.name.eq(*v))
            .is_some()
    }
}

impl TypeDefinitionExtension for CompositeType {
    fn is_leaf_type(&self) -> bool {
        false
    }

    fn is_composite_type(&self) -> bool {
        true
    }

    fn is_input_type(&self) -> bool {
        false
    }

    fn name(&self) -> String {
        match self {
            CompositeType::Object(o) => o.name.clone(),
            CompositeType::Interface(i) => i.name.clone(),
            CompositeType::Union(u) => u.name.clone(),
        }
    }

    fn is_abstract_type(&self) -> bool {
        match self {
            CompositeType::Object(_o) => false,
            CompositeType::Interface(_i) => true,
            CompositeType::Union(_u) => true,
        }
    }
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

    fn is_abstract_type(&self) -> bool {
        match self {
            schema::TypeDefinition::Object(_o) => false,
            schema::TypeDefinition::Interface(_i) => true,
            schema::TypeDefinition::Union(_u) => true,
            schema::TypeDefinition::Scalar(_u) => false,
            schema::TypeDefinition::Enum(_u) => false,
            schema::TypeDefinition::InputObject(_u) => false,
        }
    }

    fn is_leaf_type(&self) -> bool {
        match self {
            schema::TypeDefinition::Object(_o) => false,
            schema::TypeDefinition::Interface(_i) => false,
            schema::TypeDefinition::Union(_u) => false,
            schema::TypeDefinition::Scalar(_u) => true,
            schema::TypeDefinition::Enum(_u) => true,
            schema::TypeDefinition::InputObject(_u) => false,
        }
    }

    fn is_input_type(&self) -> bool {
        match self {
            schema::TypeDefinition::Object(_o) => false,
            schema::TypeDefinition::Interface(_i) => false,
            schema::TypeDefinition::Union(_u) => false,
            schema::TypeDefinition::Scalar(_u) => true,
            schema::TypeDefinition::Enum(_u) => true,
            schema::TypeDefinition::InputObject(_u) => true,
        }
    }

    fn is_composite_type(&self) -> bool {
        match self {
            schema::TypeDefinition::Object(_o) => true,
            schema::TypeDefinition::Interface(_i) => true,
            schema::TypeDefinition::Union(_u) => true,
            schema::TypeDefinition::Scalar(_u) => false,
            schema::TypeDefinition::Enum(_u) => false,
            schema::TypeDefinition::InputObject(_u) => false,
        }
    }
}

pub trait AstNodeWithName {
    fn node_name(&self) -> Option<String>;
}

impl AstNodeWithName for query::OperationDefinition {
    fn node_name(&self) -> Option<String> {
        match self {
            query::OperationDefinition::Query(q) => q.name.clone(),
            query::OperationDefinition::SelectionSet(_s) => None,
            query::OperationDefinition::Mutation(m) => m.name.clone(),
            query::OperationDefinition::Subscription(s) => s.name.clone(),
        }
    }
}

impl AstNodeWithName for query::FragmentDefinition {
    fn node_name(&self) -> Option<String> {
        Some(self.name.clone())
    }
}

impl AstNodeWithName for query::FragmentSpread {
    fn node_name(&self) -> Option<String> {
        Some(self.fragment_name.clone())
    }
}

pub trait FragmentSpreadExtraction {
    fn get_recursive_fragment_spreads(&self) -> Vec<FragmentSpread>;
    fn get_fragment_spreads(&self) -> Vec<FragmentSpread>;
}

impl FragmentSpreadExtraction for query::SelectionSet {
    fn get_recursive_fragment_spreads(&self) -> Vec<FragmentSpread> {
        self.items
            .iter()
            .flat_map(|v| match v {
                query::Selection::FragmentSpread(f) => vec![f.clone()],
                query::Selection::Field(f) => f.selection_set.get_fragment_spreads(),
                query::Selection::InlineFragment(f) => f.selection_set.get_fragment_spreads(),
            })
            .collect()
    }

    fn get_fragment_spreads(&self) -> Vec<FragmentSpread> {
        self.items
            .iter()
            .flat_map(|v| match v {
                query::Selection::FragmentSpread(f) => vec![f.clone()],
                _ => vec![],
            })
            .collect()
    }
}
