use graphql_parser::query::{
    Definition, Document, Field, FragmentDefinition, FragmentSpread, InlineFragment, Mutation,
    OperationDefinition, Query, Selection, SelectionSet, Subscription, Value, VariableDefinition,
};

use super::DefaultVisitorContext;

pub trait QueryVisitor<'a, T = DefaultVisitorContext> {
    fn visit_document(&mut self, node: &'a Document<'a, String>, visitor_context: &'a T) {
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

    fn __visit_selection_set(
        &mut self,
        _node: &'a SelectionSet<'a, String>,
        visitor_context: &'a T,
    ) {
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
    }

    fn enter_document(&mut self, _node: &'a Document<'a, String>, _visitor_context: &'a T) {}
    fn leave_document(&mut self, _node: &'a Document<'a, String>, _visitor_context: &'a T) {}

    fn enter_definition(&mut self, _node: &'a Definition<'a, String>, _visitor_context: &'a T) {}
    fn leave_definition(&mut self, _node: &'a Definition<'a, String>, _visitor_context: &'a T) {}

    fn enter_fragment_definition(
        &mut self,
        _node: &'a FragmentDefinition<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
    fn leave_fragment_definition(
        &mut self,
        _node: &'a FragmentDefinition<'a, String>,
        _visitor_context: &'a T,
    ) {
    }

    fn enter_operation_definition(
        &mut self,
        _node: &'a OperationDefinition<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
    fn leave_operation_definition(
        &mut self,
        _node: &'a OperationDefinition<'a, String>,
        _visitor_context: &'a T,
    ) {
    }

    fn enter_query(&mut self, _node: &'a Query<'a, String>, _visitor_context: &'a T) {}
    fn leave_query(&mut self, _node: &'a Query<'a, String>, _visitor_context: &'a T) {}

    fn enter_mutation(&mut self, _node: &'a Mutation<'a, String>, _visitor_context: &'a T) {}
    fn leave_mutation(&mut self, _node: &'a Mutation<'a, String>, _visitor_context: &'a T) {}

    fn enter_subscription(&mut self, _node: &'a Subscription<'a, String>, _visitor_context: &'a T) {
    }
    fn leave_subscription(&mut self, _node: &'a Subscription<'a, String>, _visitor_context: &'a T) {
    }

    fn enter_selection_set(
        &mut self,
        _node: &'a SelectionSet<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
    fn leave_selection_set(
        &mut self,
        _node: &'a SelectionSet<'a, String>,
        _visitor_context: &'a T,
    ) {
    }

    fn enter_variable_definition(
        &mut self,
        _node: &'a VariableDefinition<'a, String>,
        _parent_operation: &'a OperationDefinition<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
    fn leave_variable_definition(
        &mut self,
        _node: &'a VariableDefinition<'a, String>,
        _parent_operation: &'a OperationDefinition<'a, String>,
        _visitor_context: &'a T,
    ) {
    }

    fn enter_selection(&mut self, _node: &'a Selection<'a, String>, _visitor_context: &'a T) {}
    fn leave_selection(&mut self, _node: &'a Selection<'a, String>, _visitor_context: &'a T) {}

    fn enter_field(&mut self, _node: &'a Field<'a, String>, _visitor_context: &'a T) {}
    fn leave_field(&mut self, _node: &'a Field<'a, String>, _visitor_context: &'a T) {}

    fn enter_field_argument(
        &mut self,
        _name: &'a String,
        _value: &'a Value<'a, String>,
        _parent_field: &'a Field<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
    fn leave_field_argument(
        &mut self,
        _name: &'a String,
        _value: &'a Value<'a, String>,
        _parent_field: &'a Field<'a, String>,
        _visitor_context: &'a T,
    ) {
    }

    fn enter_fragment_spread(
        &mut self,
        _node: &'a FragmentSpread<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
    fn leave_fragment_spread(
        &mut self,
        _node: &'a FragmentSpread<'a, String>,
        _visitor_context: &'a T,
    ) {
    }

    fn enter_inline_fragment(
        &mut self,
        _node: &'a InlineFragment<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
    fn leave_inline_fragment(
        &mut self,
        _node: &'a InlineFragment<'a, String>,
        _visitor_context: &'a T,
    ) {
    }
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

    struct TestVisitor {
        collected_queries: Vec<String>,
    }

    impl<'a> TestVisitor {
        fn collect_visited_info(&mut self, document: &'a Document<'a, String>) {
            self.visit_document(document, &DefaultVisitorContext {});
        }
    }

    impl<'a> QueryVisitor<'a> for TestVisitor {
        fn enter_query(&mut self, _node: &'a Query<'a, String>, ctx: &DefaultVisitorContext) {
            self.collected_queries
                .push(_node.name.as_ref().unwrap().to_string());
        }
    }

    let mut visitor = TestVisitor {
        collected_queries: Vec::new(),
    };

    visitor.collect_visited_info(&query_ast);
    assert_eq!(visitor.collected_queries, vec!["someQuery"]);
}
