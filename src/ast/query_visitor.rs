use crate::static_graphql::query::{
    Definition, Document, Field, FragmentDefinition, FragmentSpread, InlineFragment, Mutation,
    OperationDefinition, Query, Selection, SelectionSet, Subscription, Value, VariableDefinition,
};

use super::DefaultVisitorContext;

/// A trait for implenenting a visitor for GraphQL operations.
/// It allow your custom function to be called when an AST node is found.
///
/// You can pass custom <T> as context if you need to store data / access external variables.
pub trait QueryVisitor<T = DefaultVisitorContext> {
    fn visit_document(&self, node: &Document, visitor_context: &mut T) {
        self.enter_document(node, visitor_context);

        for definition in &node.definitions {
            self.enter_definition(definition, visitor_context);

            match definition {
                Definition::Fragment(fragment) => {
                    self.enter_fragment_definition(fragment, visitor_context);
                    self.__visit_selection_set(&fragment.selection_set, visitor_context);
                    self.leave_fragment_definition(fragment, visitor_context);
                }
                Definition::Operation(operation) => {
                    self.enter_operation_definition(operation, visitor_context);

                    match operation {
                        OperationDefinition::Query(query) => {
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

                            self.__visit_selection_set(&query.selection_set, visitor_context);
                            self.leave_query(query, visitor_context);
                        }
                        OperationDefinition::Mutation(mutation) => {
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
                            self.__visit_selection_set(&mutation.selection_set, visitor_context);
                            self.leave_mutation(mutation, visitor_context);
                        }
                        OperationDefinition::Subscription(subscription) => {
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
                            );
                            self.leave_subscription(subscription, visitor_context);
                        }
                        OperationDefinition::SelectionSet(selection_set) => {
                            self.enter_selection_set(selection_set, visitor_context);
                            self.__visit_selection_set(&selection_set, visitor_context);
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

    fn __visit_selection_set(&self, _node: &SelectionSet, visitor_context: &mut T) {
        self.enter_selection_set(_node, visitor_context);

        for selection in &_node.items {
            self.enter_selection(selection, visitor_context);

            match selection {
                Selection::Field(field) => {
                    self.enter_field(field, visitor_context);

                    for (name, argument) in &field.arguments {
                        self.enter_field_argument(name, argument, field, visitor_context);
                        self.leave_field_argument(name, argument, field, visitor_context);
                    }

                    self.__visit_selection_set(&field.selection_set, visitor_context);
                    self.leave_field(field, visitor_context);
                }
                Selection::FragmentSpread(fragment_spread) => {
                    self.enter_fragment_spread(fragment_spread, visitor_context);
                    self.leave_fragment_spread(fragment_spread, visitor_context);
                }
                Selection::InlineFragment(inline_fragment) => {
                    self.enter_inline_fragment(inline_fragment, visitor_context);
                    self.__visit_selection_set(&inline_fragment.selection_set, visitor_context);
                    self.leave_inline_fragment(inline_fragment, visitor_context);
                }
            }

            self.leave_selection(selection, visitor_context);
        }

        self.leave_selection_set(_node, visitor_context);
    }

    fn enter_document(&self, _node: &Document, _visitor_context: &mut T) {}
    fn leave_document(&self, _node: &Document, _visitor_context: &mut T) {}

    fn enter_definition(&self, _node: &Definition, _visitor_context: &mut T) {}
    fn leave_definition(&self, _node: &Definition, _visitor_context: &mut T) {}

    fn enter_fragment_definition(&self, _node: &FragmentDefinition, _visitor_context: &mut T) {}
    fn leave_fragment_definition(&self, _node: &FragmentDefinition, _visitor_context: &mut T) {}

    fn enter_operation_definition(&self, _node: &OperationDefinition, _visitor_context: &mut T) {}
    fn leave_operation_definition(&self, _node: &OperationDefinition, _visitor_context: &mut T) {}

    fn enter_query(&self, _node: &Query, _visitor_context: &mut T) {}
    fn leave_query(&self, _node: &Query, _visitor_context: &mut T) {}

    fn enter_mutation(&self, _node: &Mutation, _visitor_context: &mut T) {}
    fn leave_mutation(&self, _node: &Mutation, _visitor_context: &mut T) {}

    fn enter_subscription(&self, _node: &Subscription, _visitor_context: &mut T) {}
    fn leave_subscription(&self, _node: &Subscription, _visitor_context: &mut T) {}

    fn enter_selection_set(&self, _node: &SelectionSet, _visitor_context: &mut T) {}
    fn leave_selection_set(&self, _node: &SelectionSet, _visitor_context: &mut T) {}

    fn enter_variable_definition(
        &self,
        _node: &VariableDefinition,
        _parent_operation: &OperationDefinition,
        _visitor_context: &mut T,
    ) {
    }
    fn leave_variable_definition(
        &self,
        _node: &VariableDefinition,
        _parent_operation: &OperationDefinition,
        _visitor_context: &mut T,
    ) {
    }

    fn enter_selection(&self, _node: &Selection, _visitor_context: &mut T) {}
    fn leave_selection(&self, _node: &Selection, _visitor_context: &mut T) {}

    fn enter_field(&self, _node: &Field, _visitor_context: &mut T) {}
    fn leave_field(&self, _node: &Field, _visitor_context: &mut T) {}

    fn enter_field_argument(
        &self,
        _name: &String,
        _value: &Value,
        _parent_field: &Field,
        _visitor_context: &mut T,
    ) {
    }
    fn leave_field_argument(
        &self,
        _name: &String,
        _value: &Value,
        _parent_field: &Field,
        _visitor_context: &mut T,
    ) {
    }

    fn enter_fragment_spread(&self, _node: &FragmentSpread, _visitor_context: &mut T) {}
    fn leave_fragment_spread(&self, _node: &FragmentSpread, _visitor_context: &mut T) {}

    fn enter_inline_fragment(&self, _node: &InlineFragment, _visitor_context: &mut T) {}
    fn leave_inline_fragment(&self, _node: &InlineFragment, _visitor_context: &mut T) {}
}

#[test]
fn visit_test_all_nodes() {
    use graphql_parser::query::parse_query;

    let query_ast = parse_query::<String>(
        r#"query someQuery($v: String) {
      hero(v: $v, otherV: 10) {
        name
      }

      test {
        ...SpreadHere

        anotherField {
          nested {
            moreNested
          }
        }
      }

      search(term: "Test") {
        ... on SearchResult {
          result
        }
      }
    }"#,
    )
    .expect("failed to parse query");

    struct TestVisitorCollected {
        collected_queries: Vec<String>,
    }

    struct TestVisitor;

    impl TestVisitor {
        fn collect_visited_info(&self, document: &Document, collector: &mut TestVisitorCollected) {
            self.visit_document(document, collector);
        }
    }

    impl QueryVisitor<TestVisitorCollected> for TestVisitor {
        fn enter_query(&self, _node: &Query, _ctx: &mut TestVisitorCollected) {
            _ctx.collected_queries
                .push(_node.name.as_ref().unwrap().to_string());
        }
    }

    let mut collector = TestVisitorCollected {
        collected_queries: Vec::new(),
    };

    let visitor = TestVisitor {};

    visitor.collect_visited_info(&query_ast, &mut collector);
    assert_eq!(collector.collected_queries, vec!["someQuery"]);
}
