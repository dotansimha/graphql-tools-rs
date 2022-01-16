use std::collections::{BTreeMap, HashMap};

use graphql_parser::query::TypeCondition;

use crate::static_graphql::{
    query::*,
    schema::{self, DirectiveDefinition, Field as SchemaFieldDef, InputValue},
    schema::{Document as SchemaDocument, ObjectType, TypeDefinition},
};

use crate::ast::ext::TypeDefinitionExtension;

use super::AstTypeRef;

/// Extensions
///

pub trait FieldByNameExtension {
    fn field_by_name(&self, name: &String) -> Option<SchemaFieldDef>;
    fn input_field_by_name(&self, name: &String) -> Option<InputValue>;
}

impl FieldByNameExtension for TypeDefinition {
    fn field_by_name(&self, name: &String) -> Option<SchemaFieldDef> {
        match self {
            TypeDefinition::Object(object) => object
                .fields
                .iter()
                .find(|field| field.name.eq(name))
                .cloned(),
            TypeDefinition::Interface(interface) => interface
                .fields
                .iter()
                .find(|field| field.name.eq(name))
                .cloned(),
            _ => None,
        }
    }

    fn input_field_by_name(&self, name: &String) -> Option<InputValue> {
        match self {
            TypeDefinition::InputObject(input_object) => input_object
                .fields
                .iter()
                .find(|field| field.name.eq(name))
                .cloned(),
            _ => None,
        }
    }
}

trait OperationDefinitionExtension {
    fn variable_definitions(&self) -> &[VariableDefinition];
    fn selection_set(&self) -> &SelectionSet;
}

impl OperationDefinitionExtension for OperationDefinition {
    fn variable_definitions(&self) -> &[VariableDefinition] {
        match self {
            OperationDefinition::Query(query) => &query.variable_definitions,
            OperationDefinition::SelectionSet(_) => &[],
            OperationDefinition::Mutation(mutation) => &mutation.variable_definitions,
            OperationDefinition::Subscription(subscription) => &subscription.variable_definitions,
        }
    }

    fn selection_set(&self) -> &SelectionSet {
        match self {
            OperationDefinition::Query(query) => &query.selection_set,
            OperationDefinition::SelectionSet(selection_set) => &selection_set,
            OperationDefinition::Mutation(mutation) => &mutation.selection_set,
            OperationDefinition::Subscription(subscription) => &subscription.selection_set,
        }
    }
}

pub trait SchemaDocumentExtension {
    fn type_by_name(&self, name: &String) -> Option<TypeDefinition>;
    fn type_map(&self) -> HashMap<String, TypeDefinition>;
    fn directive_by_name(&self, name: &String) -> Option<DirectiveDefinition>;
    fn object_type_by_name(&self, name: &String) -> Option<ObjectType>;
    fn schema_definition(&self) -> schema::SchemaDefinition;
    fn query_type(&self) -> ObjectType;
    fn mutation_type(&self) -> Option<ObjectType>;
    fn subscription_type(&self) -> Option<ObjectType>;
}

impl SchemaDocumentExtension for SchemaDocument {
    fn type_by_name(&self, name: &String) -> Option<TypeDefinition> {
        for def in &self.definitions {
            if let schema::Definition::TypeDefinition(type_def) = def {
                if type_def.name().eq(name) {
                    return Some(type_def.clone());
                }
            }
        }

        None
    }

    fn directive_by_name(&self, name: &String) -> Option<DirectiveDefinition> {
        for def in &self.definitions {
            if let schema::Definition::DirectiveDefinition(directive_def) = def {
                if directive_def.name.eq(name) {
                    return Some(directive_def.clone());
                }
            }
        }

        None
    }

    fn schema_definition(&self) -> schema::SchemaDefinition {
        self.definitions
            .iter()
            .find_map(|definition| match definition {
                schema::Definition::SchemaDefinition(schema_definition) => {
                    Some(schema_definition.clone())
                }
                _ => None,
            })
            .unwrap_or(schema::SchemaDefinition {
                query: Some("Query".to_string()),
                ..Default::default()
            })
    }

    fn query_type(&self) -> ObjectType {
        let schema_definition = self.schema_definition();

        self.object_type_by_name(
            schema_definition
                .query
                .as_ref()
                .unwrap_or(&"Query".to_string()),
        )
        .unwrap()
    }

    fn mutation_type(&self) -> Option<ObjectType> {
        self.schema_definition()
            .mutation
            .and_then(|name| self.object_type_by_name(&name))
    }

    fn subscription_type(&self) -> Option<ObjectType> {
        self.schema_definition()
            .subscription
            .and_then(|name| self.object_type_by_name(&name))
    }

    fn object_type_by_name(&self, name: &String) -> Option<ObjectType> {
        match self.type_by_name(name) {
            Some(TypeDefinition::Object(object_def)) => Some(object_def),
            _ => None,
        }
    }

    fn type_map(&self) -> HashMap<String, TypeDefinition> {
        let mut type_map = HashMap::new();

        for def in &self.definitions {
            if let schema::Definition::TypeDefinition(type_def) = def {
                type_map.insert(type_def.name().clone(), type_def.clone());
            }
        }

        type_map
    }
}

/// OperationVisitor
pub struct OperationVisitorContext<'a, UserContext> {
    pub user_context: &'a mut UserContext,
    pub schema: &'a SchemaDocument,
    pub known_fragments: HashMap<String, FragmentDefinition>,
    pub directives: HashMap<String, DirectiveDefinition>,

    type_stack: Vec<Option<TypeDefinition>>,
    parent_type_stack: Vec<Option<TypeDefinition>>,
    input_type_stack: Vec<Option<TypeDefinition>>,
    type_literal_stack: Vec<Option<Type>>,
    input_type_literal_stack: Vec<Option<Type>>,
}

impl<'a, UserContext> OperationVisitorContext<'a, UserContext> {
    pub fn new(
        user_context: &'a mut UserContext,
        operation: &'a Document,
        schema: &'a SchemaDocument,
    ) -> Self {
        OperationVisitorContext {
            user_context,
            schema,
            type_stack: vec![],
            parent_type_stack: vec![],
            input_type_stack: vec![],
            type_literal_stack: vec![],
            input_type_literal_stack: vec![],
            known_fragments: HashMap::<String, FragmentDefinition>::from_iter(
                operation.definitions.iter().filter_map(|def| match def {
                    Definition::Fragment(fragment) => {
                        Some((fragment.name.clone(), fragment.clone()))
                    }
                    _ => None,
                }),
            ),
            directives: HashMap::<String, DirectiveDefinition>::from_iter(
                schema.definitions.iter().filter_map(|def| match def {
                    schema::Definition::DirectiveDefinition(directive_def) => {
                        Some((directive_def.name.clone(), directive_def.clone()))
                    }
                    _ => None,
                }),
            ),
        }
    }

    pub fn with_type<Func>(&mut self, t: Option<Type>, func: Func)
    where
        Func: FnOnce(&mut OperationVisitorContext<'a, UserContext>) -> (),
    {
        if let Some(ref t) = t {
            self.type_stack
                .push(self.schema.type_by_name(&t.named_type()));
        } else {
            self.type_stack.push(None);
        }

        self.type_literal_stack.push(t);
        func(self);
        self.type_literal_stack.pop();
        self.type_stack.pop();
    }

    pub fn with_parent_type<Func>(&mut self, func: Func)
    where
        Func: FnOnce(&mut OperationVisitorContext<'a, UserContext>) -> (),
    {
        self.parent_type_stack
            .push(self.type_stack.last().unwrap_or(&None).clone());
        func(self);
        self.parent_type_stack.pop();
    }

    pub fn with_input_type<Func>(&mut self, t: Option<Type>, func: Func)
    where
        Func: FnOnce(&mut OperationVisitorContext<'a, UserContext>) -> (),
    {
        if let Some(ref t) = t {
            self.input_type_stack
                .push(self.schema.type_by_name(&t.named_type()));
        } else {
            self.input_type_stack.push(None);
        }

        self.input_type_literal_stack.push(t);
        func(self);
        self.input_type_literal_stack.pop();
        self.input_type_stack.pop();
    }

    pub fn current_type(&self) -> Option<&TypeDefinition> {
        self.type_stack.last().unwrap_or(&None).as_ref()
    }

    pub fn current_parent_type(&self) -> Option<&TypeDefinition> {
        self.parent_type_stack.last().unwrap_or(&None).as_ref()
    }

    pub fn current_type_literal(&self) -> Option<&Type> {
        self.type_literal_stack.last().unwrap_or(&None).as_ref()
    }

    pub fn current_input_type_literal(&self) -> Option<&Type> {
        self.input_type_literal_stack
            .last()
            .unwrap_or(&None)
            .as_ref()
    }
}

pub fn visit_document<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    document: &Document,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    visitor.enter_document(context, document);
    visit_definitions(visitor, &document.definitions, context);
    visitor.leave_document(context, document);
}

fn visit_definitions<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    definitions: &Vec<Definition>,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    for definition in definitions {
        let schema_type_name = match definition {
            Definition::Fragment(fragment) => {
                let TypeCondition::On(name) = &fragment.type_condition;
                Some(name.clone())
            }
            Definition::Operation(operation) => match operation {
                OperationDefinition::Query(_) => Some(context.schema.query_type().name.clone()),
                OperationDefinition::SelectionSet(_) => {
                    Some(context.schema.query_type().name.clone())
                }
                OperationDefinition::Mutation(_) => context.schema.mutation_type().map(|t| t.name),
                OperationDefinition::Subscription(_) => {
                    context.schema.subscription_type().map(|t| t.name)
                }
            },
        };

        context.with_type(
            schema_type_name.map(|v| Type::NamedType(v)),
            |context| match definition {
                Definition::Fragment(fragment) => {
                    visit_fragment_definition(visitor, fragment, context)
                }
                Definition::Operation(operation) => {
                    visit_operation_definition(visitor, operation, context)
                }
            },
        );
    }
}

fn visit_directives<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    directives: &Vec<Directive>,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    for directive in directives {
        let directive_def_args = context
            .schema
            .directive_by_name(&directive.name)
            .map(|def| def.arguments);

        visitor.enter_directive(context, directive);
        visit_arguments(
            visitor,
            directive_def_args.as_ref(),
            &directive.arguments,
            context,
        );
        visitor.leave_directive(context, directive);
    }
}

fn visit_arguments<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    arguments_definition: Option<&Vec<InputValue>>,
    arguments: &Vec<(String, Value)>,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    for argument in arguments {
        let arg_type = arguments_definition
            .and_then(|argument_defs| argument_defs.iter().find(|a| a.name.eq(&argument.0)))
            .map(|a| a.value_type.clone());

        context.with_input_type(arg_type, |context| {
            visitor.enter_argument(context, argument);
            visit_input_value(visitor, &argument.1, context);
            visitor.leave_argument(context, argument);
        })
    }
}

fn visit_input_value<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    input_value: &Value,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    match input_value {
        Value::Boolean(v) => {
            visitor.enter_scalar_value(context, v);
            visitor.leave_scalar_value(context, v);
        }
        Value::Float(v) => {
            visitor.enter_scalar_value(context, v);
            visitor.leave_scalar_value(context, v);
        }
        Value::Int(v) => {
            visitor.enter_scalar_value(context, v);
            visitor.leave_scalar_value(context, v);
        }
        Value::Null => {
            visitor.enter_null_value(context, ());
            visitor.leave_null_value(context, ());
        }
        Value::String(v) => {
            visitor.enter_scalar_value(context, v);
            visitor.leave_scalar_value(context, v);
        }
        Value::Enum(v) => {
            visitor.enter_enum_value(context, v.clone());
            visitor.leave_enum_value(context, v.clone());
        }
        Value::List(v) => {
            visitor.enter_list_value(context, v.clone());

            let input_type = context.current_input_type_literal().and_then(|t| match t {
                Type::ListType(inner_type) => Some(inner_type.as_ref().clone()),
                _ => None,
            });

            context.with_input_type(input_type, |context| {
                for item in v {
                    visit_input_value(visitor, item, context)
                }
            });

            visitor.leave_list_value(context, v.clone());
        }
        Value::Object(v) => {
            visitor.enter_object_value(context, v.clone());

            for (sub_key, sub_value) in v.iter() {
                let input_type = context
                    .current_input_type_literal()
                    .and_then(|v| context.schema.type_by_name(&v.named_type()))
                    .and_then(|v| v.input_field_by_name(&sub_key))
                    .and_then(|v| Some(v.value_type));

                context.with_input_type(input_type, |context| {
                    let param = &(sub_key.clone(), sub_value.clone());
                    visitor.enter_object_field(context, param);
                    visit_input_value(visitor, sub_value, context);
                    visitor.leave_object_field(context, param);
                });
            }

            visitor.leave_object_value(context, v.clone());
        }
        Value::Variable(v) => {
            visitor.enter_variable_value(context, v.clone());
            visitor.leave_variable_value(context, v.clone());
        }
    }
}

fn visit_variable_definitions<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    variables: &[VariableDefinition],
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    for variable in variables {
        context.with_input_type(Some(variable.var_type.clone()), |context| {
            visitor.enter_variable_definition(context, variable);

            if let Some(default_value) = &variable.default_value {
                visit_input_value(visitor, &default_value, context);
            }

            // DOTAN: We should visit the directives as well here, but it's extracted in graphql_parser.

            visitor.leave_variable_definition(context, variable);
        })
    }
}

fn visit_selection<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    selection: &Selection,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    match selection {
        Selection::Field(field) => {
            let parent_type_def = context
                .current_parent_type()
                .and_then(|t| t.field_by_name(&field.name));

            let field_type = parent_type_def.clone().map(|f| f.field_type);
            let field_args = parent_type_def.map(|f| f.arguments);

            context.with_type(field_type, |context| {
                visitor.enter_field(context, field);
                visit_arguments(visitor, field_args.as_ref(), &field.arguments, context);
                visit_directives(visitor, &field.directives, context);
                visit_selection_set(visitor, &field.selection_set, context);
                visitor.leave_field(context, field);
            });
        }
        Selection::FragmentSpread(fragment_spread) => {
            visitor.enter_fragment_spread(context, fragment_spread);
            visit_directives(visitor, &fragment_spread.directives, context);
            visitor.leave_fragment_spread(context, fragment_spread);
        }
        Selection::InlineFragment(inline_fragment) => {
            if let Some(TypeCondition::On(fragment_condition)) = &inline_fragment.type_condition {
                context.with_type(
                    Some(Type::NamedType(fragment_condition.clone())),
                    |context| {
                        visitor.enter_inline_fragment(context, inline_fragment);
                        visit_directives(visitor, &inline_fragment.directives, context);
                        visit_selection_set(visitor, &inline_fragment.selection_set, context);
                        visitor.leave_inline_fragment(context, inline_fragment);
                    },
                );
            } else {
                visitor.enter_inline_fragment(context, inline_fragment);
                visit_directives(visitor, &inline_fragment.directives, context);
                visit_selection_set(visitor, &inline_fragment.selection_set, context);
                visitor.leave_inline_fragment(context, inline_fragment);
            }
        }
    }
}

fn visit_selection_set<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    selection_set: &SelectionSet,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    context.with_parent_type(|context| {
        visitor.enter_selection_set(context, selection_set);

        for selection in &selection_set.items {
            visit_selection(visitor, selection, context);
        }

        visitor.leave_selection_set(context, selection_set);
    });
}

fn visit_fragment_definition<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    fragment: &FragmentDefinition,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    visitor.enter_fragment_definition(context, fragment);
    visit_directives(visitor, &fragment.directives, context);
    visit_selection_set(visitor, &fragment.selection_set, context);
    visitor.leave_fragment_definition(context, fragment);
}

fn visit_operation_definition<'a, Visitor, UserContext>(
    visitor: &mut Visitor,
    operation: &OperationDefinition,
    context: &mut OperationVisitorContext<'a, UserContext>,
) where
    Visitor: OperationVisitor<'a, UserContext>,
{
    visitor.enter_operation_definition(context, operation);
    // DOTAN: Maybe we need to iterate directives as well? but i think graphql_parser does not have it at the moment?
    visit_variable_definitions(visitor, operation.variable_definitions(), context);
    visit_selection_set(visitor, operation.selection_set(), context);
    visitor.leave_operation_definition(context, operation);
}

// Trait
pub trait OperationVisitor<'a, Context> {
    fn enter_document(&mut self, _: &mut OperationVisitorContext<Context>, _: &Document) {}
    fn leave_document(&mut self, _: &mut OperationVisitorContext<Context>, _: &Document) {}

    fn enter_operation_definition(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &OperationDefinition,
    ) {
    }
    fn leave_operation_definition(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &OperationDefinition,
    ) {
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &FragmentDefinition,
    ) {
    }
    fn leave_fragment_definition(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &FragmentDefinition,
    ) {
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &VariableDefinition,
    ) {
    }
    fn leave_variable_definition(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &VariableDefinition,
    ) {
    }

    fn enter_directive(&mut self, _: &mut OperationVisitorContext<Context>, _: &Directive) {}
    fn leave_directive(&mut self, _: &mut OperationVisitorContext<Context>, _: &Directive) {}

    fn enter_argument(&mut self, _: &mut OperationVisitorContext<Context>, _: &(String, Value)) {}
    fn leave_argument(&mut self, _: &mut OperationVisitorContext<Context>, _: &(String, Value)) {}

    fn enter_selection_set(&mut self, _: &mut OperationVisitorContext<Context>, _: &SelectionSet) {}
    fn leave_selection_set(&mut self, _: &mut OperationVisitorContext<Context>, _: &SelectionSet) {}

    fn enter_field(&mut self, _: &mut OperationVisitorContext<Context>, _: &Field) {}
    fn leave_field(&mut self, _: &mut OperationVisitorContext<Context>, _: &Field) {}

    fn enter_fragment_spread(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &FragmentSpread,
    ) {
    }
    fn leave_fragment_spread(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &FragmentSpread,
    ) {
    }

    fn enter_inline_fragment(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &InlineFragment,
    ) {
    }
    fn leave_inline_fragment(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &InlineFragment,
    ) {
    }

    fn enter_null_value(&mut self, _: &mut OperationVisitorContext<Context>, _: ()) {}
    fn leave_null_value(&mut self, _: &mut OperationVisitorContext<Context>, _: ()) {}

    fn enter_scalar_value<T>(&mut self, _: &mut OperationVisitorContext<Context>, _: T) {}
    fn leave_scalar_value<T>(&mut self, _: &mut OperationVisitorContext<Context>, _: T) {}

    fn enter_enum_value(&mut self, _: &mut OperationVisitorContext<Context>, _: String) {}
    fn leave_enum_value(&mut self, _: &mut OperationVisitorContext<Context>, _: String) {}

    fn enter_variable_value(&mut self, _: &mut OperationVisitorContext<Context>, _: String) {}
    fn leave_variable_value(&mut self, _: &mut OperationVisitorContext<Context>, _: String) {}

    fn enter_list_value(&mut self, _: &mut OperationVisitorContext<Context>, _: Vec<Value>) {}
    fn leave_list_value(&mut self, _: &mut OperationVisitorContext<Context>, _: Vec<Value>) {}

    fn enter_object_value(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: BTreeMap<String, Value>,
    ) {
    }
    fn leave_object_value(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: BTreeMap<String, Value>,
    ) {
    }

    fn enter_object_field(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &(String, Value),
    ) {
    }
    fn leave_object_field(
        &mut self,
        _: &mut OperationVisitorContext<Context>,
        _: &(String, Value),
    ) {
    }
}
