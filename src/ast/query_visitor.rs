use graphql_parser::query::{
    Definition, Document, Field, FragmentDefinition, FragmentSpread, InlineFragment, Mutation,
    OperationDefinition, Query, Selection, SelectionSet, Subscription, Value, VariableDefinition,
};

pub trait QueryVisitor<'a> {
    fn __visit_document(&mut self, node: &'a Document<'a, String>) {
        self.enter_document(node);

        for definition in &node.definitions {
            self.enter_definition(definition);

            match definition {
                Definition::Fragment(fragment) => {
                    self.enter_fragment_definition(fragment);
                    self.__visit_selection_set(&fragment.selection_set);
                    self.leave_fragment_definition(fragment);
                }
                Definition::Operation(operation) => {
                    self.enter_operation_definition(operation);

                    match operation {
                        OperationDefinition::Query(query) => {
                            self.enter_query(query);

                            for variable in &query.variable_definitions {
                                self.enter_variable_definition(variable, operation);
                                self.leave_variable_definition(variable, operation);
                            }

                            self.__visit_selection_set(&query.selection_set);
                            self.leave_query(query);
                        }
                        OperationDefinition::Mutation(mutation) => {
                            self.enter_mutation(mutation);
                            for variable in &mutation.variable_definitions {
                                self.enter_variable_definition(variable, operation);
                                self.leave_variable_definition(variable, operation);
                            }
                            self.__visit_selection_set(&mutation.selection_set);
                            self.leave_mutation(mutation);
                        }
                        OperationDefinition::Subscription(subscription) => {
                            self.enter_subscription(subscription);
                            for variable in &subscription.variable_definitions {
                                self.enter_variable_definition(variable, operation);
                                self.leave_variable_definition(variable, operation);
                            }
                            self.__visit_selection_set(&subscription.selection_set);
                            self.leave_subscription(subscription);
                        }
                        OperationDefinition::SelectionSet(selection_set) => {
                            self.enter_selection_set(selection_set);
                            self.__visit_selection_set(&selection_set);
                            self.leave_selection_set(selection_set);
                        }
                    }

                    self.leave_operation_definition(operation);
                }
            }

            self.leave_definition(definition);
        }

        self.leave_document(node);
    }

    fn __visit_selection_set(&mut self, _node: &'a SelectionSet<'a, String>) {
        for selection in &_node.items {
            self.enter_selection(selection);

            match selection {
                Selection::Field(field) => {
                    self.enter_field(field);

                    for (name, argument) in &field.arguments {
                        self.enter_field_argument(name, argument, field);
                        self.leave_field_argument(name, argument, field);
                    }

                    self.__visit_selection_set(&field.selection_set);
                    self.leave_field(field);
                }
                Selection::FragmentSpread(fragment_spread) => {
                    self.enter_fragment_spread(fragment_spread);
                    self.leave_fragment_spread(fragment_spread);
                }
                Selection::InlineFragment(inline_fragment) => {
                    self.enter_inline_fragment(inline_fragment);
                    self.__visit_selection_set(&inline_fragment.selection_set);
                    self.leave_inline_fragment(inline_fragment);
                }
            }

            self.leave_selection(selection);
        }
    }

    fn enter_document(&mut self, _node: &'a Document<'a, String>) {}
    fn leave_document(&mut self, _node: &'a Document<'a, String>) {}

    fn enter_definition(&mut self, _node: &'a Definition<'a, String>) {}
    fn leave_definition(&mut self, _node: &'a Definition<'a, String>) {}

    fn enter_fragment_definition(&mut self, _node: &'a FragmentDefinition<'a, String>) {}
    fn leave_fragment_definition(&mut self, _node: &'a FragmentDefinition<'a, String>) {}

    fn enter_operation_definition(&mut self, _node: &'a OperationDefinition<'a, String>) {}
    fn leave_operation_definition(&mut self, _node: &'a OperationDefinition<'a, String>) {}

    fn enter_query(&mut self, _node: &'a Query<'a, String>) {}
    fn leave_query(&mut self, _node: &'a Query<'a, String>) {}

    fn enter_mutation(&mut self, _node: &'a Mutation<'a, String>) {}
    fn leave_mutation(&mut self, _node: &'a Mutation<'a, String>) {}

    fn enter_subscription(&mut self, _node: &'a Subscription<'a, String>) {}
    fn leave_subscription(&mut self, _node: &'a Subscription<'a, String>) {}

    fn enter_selection_set(&mut self, _node: &'a SelectionSet<'a, String>) {}
    fn leave_selection_set(&mut self, _node: &'a SelectionSet<'a, String>) {}

    fn enter_variable_definition(
        &mut self,
        _node: &'a VariableDefinition<'a, String>,
        _parent_operation: &'a OperationDefinition<'a, String>,
    ) {
    }
    fn leave_variable_definition(
        &mut self,
        _node: &'a VariableDefinition<'a, String>,
        _parent_operation: &'a OperationDefinition<'a, String>,
    ) {
    }

    fn enter_selection(&mut self, _node: &'a Selection<'a, String>) {}
    fn leave_selection(&mut self, _node: &'a Selection<'a, String>) {}

    fn enter_field(&mut self, _node: &'a Field<'a, String>) {}
    fn leave_field(&mut self, _node: &'a Field<'a, String>) {}

    fn enter_field_argument(
        &mut self,
        _name: &'a String,
        _value: &'a Value<'a, String>,
        _parent_field: &'a Field<'a, String>,
    ) {
    }
    fn leave_field_argument(
        &mut self,
        _name: &'a String,
        _value: &'a Value<'a, String>,
        _parent_field: &'a Field<'a, String>,
    ) {
    }

    fn enter_fragment_spread(&mut self, _node: &'a FragmentSpread<'a, String>) {}
    fn leave_fragment_spread(&mut self, _node: &'a FragmentSpread<'a, String>) {}

    fn enter_inline_fragment(&mut self, _node: &'a InlineFragment<'a, String>) {}
    fn leave_inline_fragment(&mut self, _node: &'a InlineFragment<'a, String>) {}
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
            self.__visit_document(document);
        }
    }

    impl<'a> QueryVisitor<'a> for TestVisitor {
        fn enter_query(&mut self, _node: &'a Query<'a, String>) {
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
