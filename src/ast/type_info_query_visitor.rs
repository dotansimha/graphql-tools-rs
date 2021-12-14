use std::collections::HashMap;

use crate::{static_graphql::{query, schema}, validation::utils::{find_object_type_by_name}};

use super::DefaultVisitorContext;

pub struct TypeInfoRegistry<'a> {
  pub query_type: &'a schema::ObjectType,
  pub mutation_type: Option<&'a schema::ObjectType>,
  pub subscription_type: Option<&'a schema::ObjectType>,
  pub type_by_name: HashMap<String, &'a schema::TypeDefinition>,
  _secret: (),
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

impl TypeInfoRegistry<'static> {
  pub fn new(
    schema: &'static schema::Document
  ) -> Self {
    let schema_definition = find_schema_definition(&schema);
    let query_type = find_object_type_by_name(&schema, match schema_definition {
      Some(schema_definition) => schema_definition.query.clone().unwrap_or("Query".to_string()),
      None => "Query".to_string(),
    }).expect("Schema does not contain a Query root type");
    let mutation_type = find_object_type_by_name(&schema, match schema_definition {
      Some(schema_definition) => schema_definition.query.clone().unwrap_or("Mutation".to_string()),
      None => "Mutation".to_string(),
    });
    let subscription_type = find_object_type_by_name(&schema, match schema_definition {
      Some(schema_definition) => schema_definition.query.clone().unwrap_or("Subscription".to_string()),
      None => "Subscription".to_string(),
    });

    let type_by_name = HashMap::from_iter(
      schema.definitions.iter().filter_map(|definition| {
        match definition {
          schema::Definition::TypeDefinition(type_definition) => {
            match type_definition {
              schema::TypeDefinition::Object(object) => Some((object.name.clone(), type_definition)),
              schema::TypeDefinition::Scalar(object) => Some((object.name.clone(), type_definition)),
              schema::TypeDefinition::Interface(object) => Some((object.name.clone(), type_definition)),
              schema::TypeDefinition::InputObject(object) => Some((object.name.clone(), type_definition)),
              schema::TypeDefinition::Enum(object) => Some((object.name.clone(), type_definition)),
              schema::TypeDefinition::Union(object) => Some((object.name.clone(), type_definition)),
              _ => None
            }
          },
          _ => None
        }
      })
    );

    return TypeInfoRegistry {
      _secret: (),
      mutation_type,
      query_type,
      subscription_type,
      type_by_name,
    }
  }
}

/// A trait for implenenting a visitor for GraphQL operations.
/// Similar to QueryVisitor, but exposes an additional `type_info` method based on the GraphQL schema.
/// 
/// You can pass custom <T> as context if you need to store data / access external variables.
pub trait TypeInfoQueryVisitor<T = DefaultVisitorContext> {
    fn visit_document(&mut self, node: &query::Document, visitor_context: &mut T, type_info: &TypeInfoRegistry) {
        let mut object_type_stack: Vec<&schema::ObjectType> = Vec::new();
        self.enter_document(node, visitor_context);

        for definition in &node.definitions {
            self.enter_definition(definition, visitor_context);

            match definition {
                query::Definition::Fragment(fragment) => {
                    let query::TypeCondition::On(type_condition) = fragment.type_condition;
                    let frag_type = type_info.type_by_name.get(&type_condition).unwrap();
                    
                    match frag_type {
                        schema::TypeDefinition::Object(object) => {
                            object_type_stack.push(object);
                        },
                        _ => {}
                    }

                    self.enter_fragment_definition(fragment, visitor_context);
                    self.__visit_selection_set(&fragment.selection_set, visitor_context, &mut object_type_stack);
                    self.leave_fragment_definition(fragment, visitor_context);
                }
                query::Definition::Operation(operation) => {
                    self.enter_operation_definition(operation, visitor_context);

                    match operation {
                      query::OperationDefinition::Query(query) => {
                            self.enter_query(query, visitor_context);

                            for variable in &query.variable_definitions {
                                self.enter_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                );
                                self.leave_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                );
                            }

                            self.__visit_selection_set(&query.selection_set, visitor_context, &mut object_type_stack);
                            self.leave_query(query, visitor_context);
                        }
                        query::OperationDefinition::Mutation(mutation) => {
                            self.enter_mutation(mutation, visitor_context);
                            for variable in &mutation.variable_definitions {
                                self.enter_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                );
                                self.leave_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                );
                            }
                            self.__visit_selection_set(&mutation.selection_set, visitor_context, &mut object_type_stack);
                            self.leave_mutation(mutation, visitor_context);
                        }
                        query::OperationDefinition::Subscription(subscription) => {
                            self.enter_subscription(subscription, visitor_context);
                            for variable in &subscription.variable_definitions {
                                self.enter_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                );
                                self.leave_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                );
                            }
                            self.__visit_selection_set(
                                &subscription.selection_set,
                                visitor_context,
                                &mut object_type_stack
                            );
                            self.leave_subscription(subscription, visitor_context);
                        }
                        query::OperationDefinition::SelectionSet(selection_set) => {
                            self.enter_selection_set(selection_set, visitor_context);
                            self.__visit_selection_set(&selection_set, visitor_context, &mut object_type_stack);
                            self.leave_selection_set(selection_set, visitor_context);
                        }
                    }

                    self.leave_operation_definition(operation, visitor_context);
                }
            }

            self.leave_definition(definition, visitor_context);
        }

        self.leave_document(node, visitor_context);
    }

    fn __visit_selection_set(&self, _node: &query::SelectionSet, visitor_context: &mut T, object_type_stack: &mut Vec<&schema::ObjectType>) {
        self.enter_selection_set(_node, visitor_context);

        for selection in &_node.items {
            self.enter_selection(selection, visitor_context);

            match selection {
              query::Selection::Field(field) => {
                    self.enter_field(field, visitor_context);

                    for (name, argument) in &field.arguments {
                        self.enter_field_argument(name, argument, field, visitor_context);
                        self.leave_field_argument(name, argument, field, visitor_context);
                    }

                    self.__visit_selection_set(&field.selection_set, visitor_context, object_type_stack);
                    self.leave_field(field, visitor_context);
                }
                query::Selection::FragmentSpread(fragment_spread) => {
                    self.enter_fragment_spread(fragment_spread, visitor_context);
                    self.leave_fragment_spread(fragment_spread, visitor_context);
                }
                query::Selection::InlineFragment(inline_fragment) => {
                    self.enter_inline_fragment(inline_fragment, visitor_context);
                    self.__visit_selection_set(&inline_fragment.selection_set, visitor_context, object_type_stack);
                    self.leave_inline_fragment(inline_fragment, visitor_context);
                }
            }

            self.leave_selection(selection, visitor_context);
        }

        self.leave_selection_set(_node, visitor_context);
    }

    fn enter_document(&self, _node: &query::Document, _visitor_context: &mut T) {}
    fn leave_document(&self, _node: &query::Document, _visitor_context: &mut T) {}

    fn enter_definition(&self, _node: &query::Definition, _visitor_context: &mut T) {}
    fn leave_definition(&self, _node: &query::Definition, _visitor_context: &mut T) {}

    fn enter_fragment_definition(&self, _node: &query::FragmentDefinition, _visitor_context: &mut T) {}
    fn leave_fragment_definition(&self, _node: &query::FragmentDefinition, _visitor_context: &mut T) {}

    fn enter_operation_definition(&self, _node: &query::OperationDefinition, _visitor_context: &mut T) {}
    fn leave_operation_definition(&self, _node: &query::OperationDefinition, _visitor_context: &mut T) {}

    fn enter_query(&self, _node: &query::Query, _visitor_context: &mut T) {}
    fn leave_query(&self, _node: &query::Query, _visitor_context: &mut T) {}

    fn enter_mutation(&self, _node: &query::Mutation, _visitor_context: &mut T) {}
    fn leave_mutation(&self, _node: &query::Mutation, _visitor_context: &mut T) {}

    fn enter_subscription(&self, _node: &query::Subscription, _visitor_context: &mut T) {}
    fn leave_subscription(&self, _node: &query::Subscription, _visitor_context: &mut T) {}

    fn enter_selection_set(&self, _node: &query::SelectionSet, _visitor_context: &mut T) {}
    fn leave_selection_set(&self, _node: &query::SelectionSet, _visitor_context: &mut T) {}

    fn enter_variable_definition(
        &self,
        _node: &query::VariableDefinition,
        _parent_operation: &query::OperationDefinition,
        _visitor_context: &T,
    ) {
    }
    fn leave_variable_definition(
        &self,
        _node: &query::VariableDefinition,
        _parent_operation: &query::OperationDefinition,
        _visitor_context: &T,
    ) {
    }

    fn enter_selection(&self, _node: &query::Selection, _visitor_context: &mut T) {}
    fn leave_selection(&self, _node: &query::Selection, _visitor_context: &mut T) {}

    fn enter_field(&self, _node: &query::Field, _visitor_context: &mut T) {}
    fn leave_field(&self, _node: &query::Field, _visitor_context: &mut T) {}

    fn enter_field_argument(
        &self,
        _name: &String,
        _value: &query::Value,
        _parent_field: &query::Field,
        _visitor_context: &T,
    ) {
    }
    fn leave_field_argument(
        &self,
        _name: &String,
        _value: &query::Value,
        _parent_field: &query::Field,
        _visitor_context: &T,
    ) {
    }

    fn enter_fragment_spread(&self, _node: &query::FragmentSpread, _visitor_context: &mut T) {}
    fn leave_fragment_spread(&self, _node: &query::FragmentSpread, _visitor_context: &mut T) {}

    fn enter_inline_fragment(&self, _node: &query::InlineFragment, _visitor_context: &mut T) {}
    fn leave_inline_fragment(&self, _node: &query::InlineFragment, _visitor_context: &mut T) {}
}
