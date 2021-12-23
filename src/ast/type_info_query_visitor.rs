use std::collections::HashMap;

use crate::{
    static_graphql::{
        query::{self, Type},
        schema,
    },
    validation::utils::find_object_type_by_name,
};

use super::{CompositeType, DefaultVisitorContext};

#[derive(Debug)]
pub struct TypeInfoRegistry<'a> {
    pub query_type: &'a schema::ObjectType,
    pub mutation_type: Option<&'a schema::ObjectType>,
    pub subscription_type: Option<&'a schema::ObjectType>,
    pub type_by_name: HashMap<String, &'a schema::TypeDefinition>,
}

fn find_schema_definition(schema: &schema::Document) -> Option<&schema::SchemaDefinition> {
    schema
        .definitions
        .iter()
        .find_map(|definition| match definition {
            schema::Definition::SchemaDefinition(schema_definition) => Some(schema_definition),
            _ => None,
        })
}

fn get_named_type(t: &Type) -> String {
    match t {
        Type::NamedType(name) => name.clone(),
        Type::ListType(inner_type) => get_named_type(inner_type),
        Type::NonNullType(inner_type) => get_named_type(inner_type),
    }
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
                        _ => None,
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

    // fn get_composite_type_by_name(name: )
}

pub struct TypeInfo {
    pub type_stack: Vec<schema::Type>,
    pub parent_type_stack: Vec<CompositeType>,
    pub input_type_stack: Vec<schema::InputObjectType>,
    pub field_def_stack: Vec<schema::Field>,
}

impl TypeInfo {
    pub fn new() -> Self {
        return TypeInfo {
            type_stack: Vec::new(),
            parent_type_stack: Vec::new(),
            input_type_stack: Vec::new(),
            field_def_stack: Vec::new(),
        };
    }

    fn get_type(&self) -> Option<schema::Type> {
        self.type_stack.last().cloned()
    }

    fn enter_type(&mut self, object: schema::Type) {
        self.type_stack.push(object);
    }

    fn leave_type(&mut self) {
        self.type_stack.pop();
    }

    fn get_parent_type(&self) -> Option<CompositeType> {
        self.parent_type_stack.last().cloned()
    }

    fn enter_parent_type(&mut self, object: CompositeType) {
        self.parent_type_stack.push(object);
    }

    fn leave_parent_type(&mut self) {
        self.parent_type_stack.pop();
    }

    fn get_field_def(&self) -> Option<schema::Field> {
        self.field_def_stack.last().cloned()
    }

    fn enter_field_def(&mut self, field: schema::Field) {
        self.field_def_stack.push(field);
    }

    fn leave_field_def(&mut self) {
        self.field_def_stack.pop();
    }
}

/// A trait for implenenting a visitor for GraphQL operations.
/// Similar to QueryVisitor, but exposes an additional `type_info` method based on the GraphQL schema.
///
/// You can pass custom <T> as context if you need to store data / access external variables.
pub trait TypeInfoQueryVisitor<T = DefaultVisitorContext> {
    fn visit_document(
        &mut self,
        node: &query::Document,
        visitor_context: &mut T,
        type_info_registry: &TypeInfoRegistry,
    ) {
        let mut type_info = TypeInfo::new();
        self.enter_document(node, visitor_context);

        for definition in &node.definitions {
            self.enter_definition(definition, visitor_context);

            match definition {
                query::Definition::Fragment(fragment) => {
                    let query::TypeCondition::On(type_condition) = fragment.type_condition.clone();
                    let frag_type = type_info_registry
                        .type_by_name
                        .get(&type_condition)
                        .unwrap();

                    match frag_type {
                        schema::TypeDefinition::Object(object) => {
                            type_info.enter_type(Type::NamedType(object.name.clone()));
                        }
                        _ => {}
                    }

                    self.enter_fragment_definition(fragment, visitor_context);
                    self.__visit_selection_set(
                        &fragment.selection_set,
                        visitor_context,
                        type_info_registry,
                        &mut type_info,
                    );
                    self.leave_fragment_definition(fragment, visitor_context);
                    type_info.leave_type();
                }
                query::Definition::Operation(operation) => {
                    self.enter_operation_definition(operation, visitor_context);

                    match operation {
                        query::OperationDefinition::Query(query) => {
                            type_info.enter_type(Type::NamedType(
                                type_info_registry.query_type.name.clone(),
                            ));
                            self.enter_query(query, visitor_context);

                            for variable in &query.variable_definitions {
                                self.enter_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &mut type_info,
                                );
                                self.leave_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &mut type_info,
                                );
                            }

                            self.__visit_selection_set(
                                &query.selection_set,
                                visitor_context,
                                type_info_registry,
                                &mut type_info,
                            );
                            self.leave_query(query, visitor_context);
                            type_info.leave_type();
                        }
                        query::OperationDefinition::Mutation(mutation) => {
                            type_info.enter_type(Type::NamedType(
                                type_info_registry.mutation_type.unwrap().name.clone(),
                            ));
                            self.enter_mutation(mutation, visitor_context);
                            for variable in &mutation.variable_definitions {
                                self.enter_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &mut type_info,
                                );
                                self.leave_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &mut type_info,
                                );
                            }
                            self.__visit_selection_set(
                                &mutation.selection_set,
                                visitor_context,
                                type_info_registry,
                                &mut type_info,
                            );
                            self.leave_mutation(mutation, visitor_context);
                            type_info.leave_type();
                        }
                        query::OperationDefinition::Subscription(subscription) => {
                            type_info.enter_type(Type::NamedType(
                                type_info_registry.subscription_type.unwrap().name.clone(),
                            ));
                            self.enter_subscription(subscription, visitor_context);
                            for variable in &subscription.variable_definitions {
                                self.enter_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &mut type_info,
                                );
                                self.leave_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &mut type_info,
                                );
                            }
                            self.__visit_selection_set(
                                &subscription.selection_set,
                                visitor_context,
                                type_info_registry,
                                &mut type_info,
                            );
                            self.leave_subscription(subscription, visitor_context);
                            type_info.leave_type();
                        }
                        query::OperationDefinition::SelectionSet(selection_set) => {
                            type_info.enter_type(Type::NamedType(
                                type_info_registry.query_type.name.clone(),
                            ));
                            self.enter_selection_set(
                                selection_set,
                                visitor_context,
                                &mut type_info,
                            );
                            self.__visit_selection_set(
                                &selection_set,
                                visitor_context,
                                type_info_registry,
                                &mut type_info,
                            );
                            self.leave_selection_set(
                                selection_set,
                                visitor_context,
                                &mut type_info,
                            );
                            type_info.leave_type();
                        }
                    }

                    self.leave_operation_definition(operation, visitor_context);
                }
            }

            self.leave_definition(definition, visitor_context);
        }

        self.leave_document(node, visitor_context);
    }

    fn __visit_selection_set(
        &self,
        _node: &query::SelectionSet,
        visitor_context: &mut T,
        type_info_registry: &TypeInfoRegistry,
        type_info: &mut TypeInfo,
    ) {
        let current_type_name = type_info
            .get_type()
            .expect("Found a field without a parent type available.");
        let named_type_name = get_named_type(&current_type_name);
        let schema_type = type_info_registry
            .type_by_name
            .get(&named_type_name)
            .expect("Found a selection set without a parent type available.");
        let as_composite_type = CompositeType::from_type_definition(schema_type);

        match as_composite_type {
            Some(t) => {
                type_info.enter_parent_type(t);
            }
            None => {}
        }

        self.enter_selection_set(_node, visitor_context, type_info);

        for selection in &_node.items {
            self.enter_selection(selection, visitor_context, type_info);

            match selection {
                query::Selection::Field(field) => {
                    let parent_type = type_info
                        .get_parent_type()
                        .expect("Found a field without a parent type available.");
                    let field_def = parent_type
                        .find_field(field.name.clone())
                        .expect("Found not found within parent type.");
                    type_info.enter_type(field_def.field_type.clone());
                    type_info.enter_field_def(field_def.clone());
                    self.enter_field(field, visitor_context, type_info);

                    for (name, argument) in &field.arguments {
                        self.enter_field_argument(
                            name,
                            argument,
                            field,
                            visitor_context,
                            type_info,
                        );
                        self.leave_field_argument(
                            name,
                            argument,
                            field,
                            visitor_context,
                            type_info,
                        );
                    }

                    self.__visit_selection_set(
                        &field.selection_set,
                        visitor_context,
                        type_info_registry,
                        type_info,
                    );
                    self.leave_field(field, visitor_context, type_info);
                    type_info.leave_field_def();
                    type_info.leave_type();
                }
                query::Selection::FragmentSpread(fragment_spread) => {
                    self.enter_fragment_spread(fragment_spread, visitor_context, type_info);
                    self.leave_fragment_spread(fragment_spread, visitor_context, type_info);
                }
                query::Selection::InlineFragment(inline_fragment) => {
                    self.enter_inline_fragment(inline_fragment, visitor_context, type_info);
                    self.__visit_selection_set(
                        &inline_fragment.selection_set,
                        visitor_context,
                        type_info_registry,
                        type_info,
                    );
                    self.leave_inline_fragment(inline_fragment, visitor_context, type_info);
                }
            }

            self.leave_selection(selection, visitor_context, type_info);
        }

        self.leave_selection_set(_node, visitor_context, type_info);
        type_info.leave_parent_type();
    }

    fn enter_document(&self, _node: &query::Document, _visitor_context: &mut T) {}
    fn leave_document(&self, _node: &query::Document, _visitor_context: &mut T) {}

    fn enter_definition(&self, _node: &query::Definition, _visitor_context: &mut T) {}
    fn leave_definition(&self, _node: &query::Definition, _visitor_context: &mut T) {}

    fn enter_fragment_definition(
        &self,
        _node: &query::FragmentDefinition,
        _visitor_context: &mut T,
    ) {
    }
    fn leave_fragment_definition(
        &self,
        _node: &query::FragmentDefinition,
        _visitor_context: &mut T,
    ) {
    }

    fn enter_operation_definition(
        &self,
        _node: &query::OperationDefinition,
        _visitor_context: &mut T,
    ) {
    }
    fn leave_operation_definition(
        &self,
        _node: &query::OperationDefinition,
        _visitor_context: &mut T,
    ) {
    }

    fn enter_query(&self, _node: &query::Query, _visitor_context: &mut T) {}
    fn leave_query(&self, _node: &query::Query, _visitor_context: &mut T) {}

    fn enter_mutation(&self, _node: &query::Mutation, _visitor_context: &mut T) {}
    fn leave_mutation(&self, _node: &query::Mutation, _visitor_context: &mut T) {}

    fn enter_subscription(&self, _node: &query::Subscription, _visitor_context: &mut T) {}
    fn leave_subscription(&self, _node: &query::Subscription, _visitor_context: &mut T) {}

    fn enter_selection_set(
        &self,
        _node: &query::SelectionSet,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }
    fn leave_selection_set(
        &self,
        _node: &query::SelectionSet,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }

    fn enter_variable_definition(
        &self,
        _node: &query::VariableDefinition,
        _parent_operation: &query::OperationDefinition,
        _visitor_context: &T,
        _type_info: &mut TypeInfo,
    ) {
    }
    fn leave_variable_definition(
        &self,
        _node: &query::VariableDefinition,
        _parent_operation: &query::OperationDefinition,
        _visitor_context: &T,
        _type_info: &mut TypeInfo,
    ) {
    }

    fn enter_selection(
        &self,
        _node: &query::Selection,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }
    fn leave_selection(
        &self,
        _node: &query::Selection,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }

    fn enter_field(
        &self,
        _node: &query::Field,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }
    fn leave_field(
        &self,
        _node: &query::Field,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }

    fn enter_field_argument(
        &self,
        _name: &String,
        _value: &query::Value,
        _parent_field: &query::Field,
        _visitor_context: &T,
        _type_info: &mut TypeInfo,
    ) {
    }
    fn leave_field_argument(
        &self,
        _name: &String,
        _value: &query::Value,
        _parent_field: &query::Field,
        _visitor_context: &T,
        _type_info: &mut TypeInfo,
    ) {
    }

    fn enter_fragment_spread(
        &self,
        _node: &query::FragmentSpread,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }
    fn leave_fragment_spread(
        &self,
        _node: &query::FragmentSpread,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }

    fn enter_inline_fragment(
        &self,
        _node: &query::InlineFragment,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }
    fn leave_inline_fragment(
        &self,
        _node: &query::InlineFragment,
        _visitor_context: &mut T,
        _type_info: &mut TypeInfo,
    ) {
    }
}

#[test]
fn should_build_correct_type_info_query() {
    use crate::validation::test_utils::TEST_SCHEMA;

    let schema_ast =
        graphql_parser::parse_schema::<String>(TEST_SCHEMA).expect("Failed to parse schema");

    let query_ast = graphql_parser::parse_query::<String>(
        r#"
        query test {
          dog {
            name
          }
        }
        "#,
    )
    .expect("failed to parse query");

    struct TestContext;
    struct TestVisitor;

    impl<'a> TypeInfoQueryVisitor<TestContext> for TestVisitor {
        fn enter_field(
            &self,
            _node: &query::Field,
            _visitor_context: &mut TestContext,
            _type_info: &mut TypeInfo,
        ) {
            println!(
                "Got into a field {}, type name: {:?}, parent type: {:?}",
                _node.name,
                _type_info.get_type(),
                _type_info.get_parent_type()
            );
        }
    }

    let mut ctx = TestContext {};
    let mut visitor = TestVisitor {};
    let type_info_registry = TypeInfoRegistry::new(&schema_ast);
    visitor.visit_document(&query_ast, &mut ctx, &type_info_registry)
}
