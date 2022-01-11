use super::{
    get_named_type, CompositeType, DefaultVisitorContext, PossibleInputType, TypeInfo,
    TypeInfoElementRef, TypeInfoRegistry,
};
use crate::static_graphql::{
    query::{self, Directive, FragmentDefinition, Type, Value},
    schema::{self, TypeDefinition},
};

use crate::ast::ext::AstTypeRef;

/// A trait for implenenting a visitor for GraphQL operations.
/// Similar to QueryVisitor, but exposes an additional `type_info` method based on the GraphQL schema.
///
/// You can pass custom <T> as context if you need to store data / access external variables.
pub trait TypeInfoQueryVisitor<T = DefaultVisitorContext> {
    fn __visit_fragment_def(
        &self,
        fragment: &FragmentDefinition,
        visitor_context: &mut T,
        type_info_registry: &TypeInfoRegistry,
        type_info: &mut TypeInfo,
    ) {
        let query::TypeCondition::On(type_condition) = fragment.type_condition.clone();
        type_info.enter_type(TypeInfoElementRef::Ref(schema::Type::NamedType(
            type_condition.clone(),
        )));

        self.enter_fragment_definition(fragment, visitor_context, &type_info);
        self.__visit_selection_set(
            &fragment.selection_set,
            visitor_context,
            type_info_registry,
            type_info,
        );
        self.leave_fragment_definition(fragment, visitor_context, &type_info);
        type_info.leave_type();
    }

    fn __visit_value(
        &self,
        arg_name: &String,
        node: &Value,
        visitor_context: &mut T,
        type_info_registry: &TypeInfoRegistry,
        type_info: &mut TypeInfo,
    ) {
        self.enter_value(node, visitor_context, type_info);

        if let Value::Object(tree_map) = node {
            tree_map.iter().for_each(|(key, sub_value)| {
                self.__visit_value(
                    key,
                    sub_value,
                    visitor_context,
                    type_info_registry,
                    type_info,
                )
            })
        }

        if let Value::Variable(var_name) = node {
            self.enter_variable(var_name, (arg_name, node), visitor_context, type_info);
            self.leave_variable(var_name, (arg_name, node), visitor_context, type_info);
        }

        self.leave_value(node, visitor_context, type_info);
    }

    fn visit_document(
        &self,
        node: &query::Document,
        visitor_context: &mut T,
        type_info_registry: &TypeInfoRegistry,
    ) {
        let mut type_info = TypeInfo::new();
        self.enter_document(node, visitor_context, &type_info);

        for definition in &node.definitions {
            self.enter_definition(definition, visitor_context, &type_info);

            match definition {
                query::Definition::Fragment(fragment) => {
                    self.__visit_fragment_def(
                        fragment,
                        visitor_context,
                        type_info_registry,
                        &mut type_info,
                    );
                }
                query::Definition::Operation(operation) => {
                    self.enter_operation_definition(operation, visitor_context, &type_info);

                    match operation {
                        query::OperationDefinition::Query(query) => {
                            type_info.enter_type(TypeInfoElementRef::Ref(Type::NamedType(
                                type_info_registry.query_type.name.clone(),
                            )));
                            self.enter_query(query, visitor_context, &type_info);

                            for variable in &query.variable_definitions {
                                match type_info_registry
                                    .type_by_name
                                    .get(&get_named_type(&variable.var_type))
                                {
                                    Some(TypeDefinition::Enum(e)) => {
                                        type_info.enter_input_type(TypeInfoElementRef::Ref(
                                            PossibleInputType::Enum(
                                                variable.var_type.clone(),
                                                e.clone(),
                                                variable.default_value.clone(),
                                            ),
                                        ));
                                    }
                                    Some(TypeDefinition::InputObject(e)) => {
                                        type_info.enter_input_type(TypeInfoElementRef::Ref(
                                            PossibleInputType::InputObject(
                                                variable.var_type.clone(),
                                                e.clone(),
                                                variable.default_value.clone(),
                                            ),
                                        ));
                                    }
                                    Some(TypeDefinition::Scalar(e)) => {
                                        type_info.enter_input_type(TypeInfoElementRef::Ref(
                                            PossibleInputType::Scalar(
                                                variable.var_type.clone(),
                                                e.clone(),
                                                variable.default_value.clone(),
                                            ),
                                        ));
                                    }
                                    _ => {
                                        type_info.enter_input_type(TypeInfoElementRef::Empty);
                                    }
                                }

                                self.enter_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &type_info,
                                );
                                self.leave_variable_definition(
                                    variable,
                                    operation,
                                    visitor_context,
                                    &type_info,
                                );

                                type_info.leave_input_type();
                            }

                            self.__visit_selection_set(
                                &query.selection_set,
                                visitor_context,
                                type_info_registry,
                                &mut type_info,
                            );
                            self.leave_query(query, visitor_context, &type_info);
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

                            self.enter_mutation(mutation, visitor_context, &type_info);
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
                            self.leave_mutation(mutation, visitor_context, &type_info);
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

                            self.enter_subscription(subscription, visitor_context, &type_info);
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
                            self.leave_subscription(subscription, visitor_context, &type_info);
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

                    self.leave_operation_definition(operation, visitor_context, &type_info);
                }
            }

            self.leave_definition(definition, visitor_context, &type_info);
        }

        self.leave_document(node, visitor_context, &type_info);
    }

    fn __visit_directive_use(
        &self,
        directive: &query::Directive,
        visitor_context: &mut T,
        _type_info_registry: &TypeInfoRegistry,
        type_info: &mut TypeInfo,
    ) {
        self.enter_directive(&directive, visitor_context, type_info);

        for (arg_name, arg_value) in &directive.arguments {
            match arg_value {
                Value::Variable(variable) => {
                    self.enter_variable(
                        variable,
                        (arg_name, arg_value),
                        visitor_context,
                        type_info,
                    );
                    self.leave_variable(
                        variable,
                        (arg_name, arg_value),
                        visitor_context,
                        type_info,
                    );
                }
                _ => {}
            }
        }

        self.leave_directive(&directive, visitor_context, type_info);
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

                    for directive in &field.directives {
                        self.__visit_directive_use(
                            directive,
                            visitor_context,
                            type_info_registry,
                            type_info,
                        );
                    }

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

                                    type_info.enter_default_value(TypeInfoElementRef::Ref(
                                        found_schema_arg.default_value.clone(),
                                    ));

                                    let arg_named_type =
                                        get_named_type(&found_schema_arg.value_type);

                                    match type_info_registry.type_by_name.get(&arg_named_type) {
                                        Some(TypeDefinition::Enum(e)) => {
                                            type_info.enter_input_type(TypeInfoElementRef::Ref(
                                                PossibleInputType::Enum(
                                                    found_schema_arg.value_type.clone(),
                                                    e.clone(),
                                                    found_schema_arg.default_value.clone(),
                                                ),
                                            ));
                                        }
                                        Some(TypeDefinition::InputObject(e)) => {
                                            type_info.enter_input_type(TypeInfoElementRef::Ref(
                                                PossibleInputType::InputObject(
                                                    found_schema_arg.value_type.clone(),
                                                    e.clone(),
                                                    found_schema_arg.default_value.clone(),
                                                ),
                                            ));
                                        }
                                        Some(TypeDefinition::Scalar(e)) => {
                                            type_info.enter_input_type(TypeInfoElementRef::Ref(
                                                PossibleInputType::Scalar(
                                                    found_schema_arg.value_type.clone(),
                                                    e.clone(),
                                                    found_schema_arg.default_value.clone(),
                                                ),
                                            ));
                                        }
                                        _ => {
                                            type_info.enter_input_type(TypeInfoElementRef::Empty);
                                        }
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

                        self.__visit_value(
                            argument_name,
                            argument_type,
                            visitor_context,
                            type_info_registry,
                            type_info,
                        );

                        match argument_type {
                            Value::Variable(variable) => {
                                self.enter_variable(
                                    variable,
                                    (argument_name, argument_type),
                                    visitor_context,
                                    type_info,
                                );
                                self.leave_variable(
                                    variable,
                                    (argument_name, argument_type),
                                    visitor_context,
                                    type_info,
                                );
                            }
                            _ => {}
                        }

                        self.leave_field_argument(
                            argument_name,
                            argument_type,
                            field,
                            visitor_context,
                            type_info,
                        );

                        type_info.leave_argument();
                        type_info.leave_default_value();
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

                    for directive in &fragment_spread.directives {
                        self.__visit_directive_use(
                            directive,
                            visitor_context,
                            type_info_registry,
                            type_info,
                        );
                    }

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

                    for directive in &inline_fragment.directives {
                        self.__visit_directive_use(
                            directive,
                            visitor_context,
                            type_info_registry,
                            type_info,
                        );
                    }

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

    fn enter_value(&self, _node: &Value, _visitor_context: &mut T, _type_info: &TypeInfo) {}
    fn leave_value(&self, _node: &Value, _visitor_context: &mut T, _type_info: &TypeInfo) {}

    fn enter_document(
        &self,
        _node: &query::Document,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_document(
        &self,
        _node: &query::Document,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_definition(
        &self,
        _node: &query::Definition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_definition(
        &self,
        _node: &query::Definition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_fragment_definition(
        &self,
        _node: &query::FragmentDefinition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_fragment_definition(
        &self,
        _node: &query::FragmentDefinition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_operation_definition(
        &self,
        _node: &query::OperationDefinition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_operation_definition(
        &self,
        _node: &query::OperationDefinition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_query(&self, _node: &query::Query, _visitor_context: &mut T, _type_info: &TypeInfo) {}
    fn leave_query(&self, _node: &query::Query, _visitor_context: &mut T, _type_info: &TypeInfo) {}

    fn enter_mutation(
        &self,
        _node: &query::Mutation,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_mutation(
        &self,
        _node: &query::Mutation,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_subscription(
        &self,
        _node: &query::Subscription,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_subscription(
        &self,
        _node: &query::Subscription,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_selection_set(
        &self,
        _node: &query::SelectionSet,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_selection_set(
        &self,
        _node: &query::SelectionSet,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_variable_definition(
        &self,
        _node: &query::VariableDefinition,
        _parent_operation: &query::OperationDefinition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_variable_definition(
        &self,
        _node: &query::VariableDefinition,
        _parent_operation: &query::OperationDefinition,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_selection(
        &self,
        _node: &query::Selection,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_selection(
        &self,
        _node: &query::Selection,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_field(&self, _node: &query::Field, _visitor_context: &mut T, _type_info: &TypeInfo) {}
    fn leave_field(&self, _node: &query::Field, _visitor_context: &mut T, _type_info: &TypeInfo) {}

    fn enter_directive(
        &self,
        _directive: &Directive,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_directive(
        &self,
        _directive: &Directive,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_field_argument(
        &self,
        _name: &String,
        _value: &query::Value,
        _parent_field: &query::Field,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_field_argument(
        &self,
        _name: &String,
        _value: &query::Value,
        _parent_field: &query::Field,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }

    fn enter_variable(
        &self,
        _name: &String,
        _parent_arg: (&String, &Value),
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_variable(
        &self,
        _name: &String,
        _parent_arg: (&String, &Value),
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
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
        _type_info: &TypeInfo,
    ) {
    }
    fn leave_inline_fragment(
        &self,
        _node: &query::InlineFragment,
        _visitor_context: &mut T,
        _type_info: &TypeInfo,
    ) {
    }
}
