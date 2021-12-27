use super::{
    get_named_type, CompositeType, DefaultVisitorContext, TypeInfo, TypeInfoElementRef,
    TypeInfoRegistry,
};
use crate::static_graphql::{
    query::{self, Type},
    schema::{self},
};

use crate::ast::ext::AstTypeRef;

/// A trait for implenenting a visitor for GraphQL operations.
/// Similar to QueryVisitor, but exposes an additional `type_info` method based on the GraphQL schema.
///
/// You can pass custom <T> as context if you need to store data / access external variables.
pub trait TypeInfoQueryVisitor<T = DefaultVisitorContext> {
    fn visit_document(
        &self,
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
                    type_info.enter_type(TypeInfoElementRef::Ref(schema::Type::NamedType(
                        type_condition.clone(),
                    )));

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
                            type_info.enter_type(TypeInfoElementRef::Ref(Type::NamedType(
                                type_info_registry.query_type.name.clone(),
                            )));
                            self.enter_query(query, visitor_context);

                            for variable in &query.variable_definitions {
                                if let Some(schema::TypeDefinition::InputObject(t)) =
                                    type_info_registry
                                        .type_by_name
                                        .get(&get_named_type(&variable.var_type))
                                {
                                    type_info.enter_input_type(TypeInfoElementRef::Ref(t.clone()));
                                } else {
                                    type_info.enter_input_type(TypeInfoElementRef::Empty);
                                }

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

                                type_info.leave_input_type();
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
                            if let Some(mutation_type) = type_info_registry.mutation_type {
                                type_info.enter_type(TypeInfoElementRef::Ref(Type::NamedType(
                                    mutation_type.name.clone(),
                                )));
                            } else {
                                type_info.enter_type(TypeInfoElementRef::Empty);
                            }

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
                            if let Some(subscription_type) = type_info_registry.subscription_type {
                                type_info.enter_type(TypeInfoElementRef::Ref(Type::NamedType(
                                    subscription_type.name.clone(),
                                )));
                            } else {
                                type_info.enter_type(TypeInfoElementRef::Empty);
                            }

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
                            type_info.enter_type(TypeInfoElementRef::Ref(Type::NamedType(
                                type_info_registry.query_type.name.clone(),
                            )));
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
        if let Some(TypeInfoElementRef::Ref(base_type)) = type_info.get_type() {
            let named_type_name = base_type.named_type();

            if let Some(type_by_name) = type_info_registry.type_by_name.get(&named_type_name) {
                if let Some(t) = CompositeType::from_type_definition(type_by_name) {
                    type_info.enter_parent_type(TypeInfoElementRef::Ref(t));
                } else {
                    type_info.enter_parent_type(TypeInfoElementRef::Empty);
                }
            } else {
                type_info.enter_parent_type(TypeInfoElementRef::Empty);
            }
        } else {
            type_info.enter_parent_type(TypeInfoElementRef::Empty);
        }

        self.enter_selection_set(_node, visitor_context, type_info);

        for selection in &_node.items {
            self.enter_selection(selection, visitor_context, type_info);

            match selection {
                query::Selection::Field(field) => {
                    if let Some(parent_type) = type_info.get_parent_type() {
                        if let Some(field_def) = parent_type.find_field(field.name.clone()) {
                            type_info
                                .enter_type(TypeInfoElementRef::Ref(field_def.field_type.clone()));
                            type_info.enter_field_def(TypeInfoElementRef::Ref(field_def.clone()));
                        } else {
                            type_info.enter_type(TypeInfoElementRef::Empty);
                            type_info.enter_field_def(TypeInfoElementRef::Empty);
                        }
                    } else {
                        type_info.enter_type(TypeInfoElementRef::Empty);
                    }

                    self.enter_field(field, visitor_context, type_info);

                    for (argument_name, argument_type) in &field.arguments {
                        if let Some(parent_type) = type_info.get_parent_type() {
                            if let Some(field_def) = parent_type.find_field(field.name.clone()) {
                                if let Some(found_schema_arg) = field_def
                                    .arguments
                                    .iter()
                                    .find(|arg| arg.name.eq(argument_name))
                                {
                                    type_info.enter_argument(TypeInfoElementRef::Ref(
                                        found_schema_arg.clone(),
                                    ));

                                    let arg_named_type =
                                        get_named_type(&found_schema_arg.value_type);

                                    if let Some(schema::TypeDefinition::InputObject(t)) =
                                        type_info_registry.type_by_name.get(&arg_named_type)
                                    {
                                        type_info
                                            .enter_input_type(TypeInfoElementRef::Ref(t.clone()));
                                    } else {
                                        type_info.enter_input_type(TypeInfoElementRef::Empty);
                                    }
                                } else {
                                    type_info.enter_argument(TypeInfoElementRef::Empty)
                                }
                            }
                        }

                        self.enter_field_argument(
                            argument_name,
                            argument_type,
                            field,
                            visitor_context,
                            type_info,
                        );
                        self.leave_field_argument(
                            argument_name,
                            argument_type,
                            field,
                            visitor_context,
                            type_info,
                        );

                        type_info.leave_argument();
                        type_info.leave_input_type();
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
                    match &inline_fragment.type_condition {
                        Some(query::TypeCondition::On(type_condition)) => {
                            type_info.enter_type(TypeInfoElementRef::Ref(schema::Type::NamedType(
                                type_condition.clone(),
                            )));
                        }
                        _ => type_info.enter_type(TypeInfoElementRef::Empty),
                    }

                    self.enter_inline_fragment(inline_fragment, visitor_context, type_info);
                    self.__visit_selection_set(
                        &inline_fragment.selection_set,
                        visitor_context,
                        type_info_registry,
                        type_info,
                    );
                    self.leave_inline_fragment(inline_fragment, visitor_context, type_info);
                    type_info.leave_type();
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
