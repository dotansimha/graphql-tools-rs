use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationError;
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};
use std::collections::HashMap;

pub struct OverlappingFieldsCanBeMerged;

struct FindOverlappingFieldsThatCanBeMerged<'a> {
    discoverd_fields: HashMap<String, Field>,
    ctx: &'a mut ValidationContext,
}

impl<'a> FindOverlappingFieldsThatCanBeMerged<'a> {
    fn store_finding(&mut self, field: &Field) {
        let field_name = field.alias.as_ref().unwrap_or(&field.name).clone();

        if let Some(existing) = self.discoverd_fields.get(&field_name) {
            if existing.name.eq(&field.name) {
                self.ctx.validation_errors.push(ValidationError {
                    locations: vec![field.position, existing.position],
                    message: format!(
                        "Fields conflict because \"{}\" and \"{}\" are different fields. Use different aliases on the fields to fetch both if this was intentional.",
                        field.name, existing.name
                    ),
                });
            }

            if existing.arguments.len() != field.arguments.len() {
                self.ctx.validation_errors.push(ValidationError {
                locations: vec![field.position, existing.position],
                message: format!(
                    "Fields \"{}\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional.",
                    field.name
                ),
              });
            }

            for (arg_name, arg_value) in &existing.arguments {
                let arg_record_in_new_field = field
                    .arguments
                    .to_owned()
                    .into_iter()
                    .find(|(arg_name_in_new_field, _)| arg_name_in_new_field == arg_name);

                match arg_record_in_new_field {
                    Some((_other_name, other_value)) if other_value.eq(arg_value) => {}
                    _ => {
                        self.ctx.validation_errors.push(ValidationError {
                        locations: vec![field.position, existing.position],
                        message: format!(
                            "Fields \"{}\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional.",
                            field.name
                        ),
                      });
                    }
                }
            }
        } else {
            self.discoverd_fields.insert(field_name, field.clone());
        }
    }

    pub fn find_in_selection_set(&mut self, selection_set: &SelectionSet) {
        for selection in &selection_set.items {
            match selection {
                Selection::Field(field) => self.store_finding(field),
                Selection::InlineFragment(inline_fragment) => {
                    self.find_in_selection_set(&inline_fragment.selection_set);
                }
                Selection::FragmentSpread(fragment_spread) => {
                    if let Some(fragment) = self
                        .ctx
                        .fragments
                        .get(&fragment_spread.fragment_name)
                        .cloned()
                    {
                        self.find_in_selection_set(&fragment.selection_set);
                    }
                }
            }
        }
    }
}

impl QueryVisitor<ValidationContext> for OverlappingFieldsCanBeMerged {
    fn enter_selection_set(&mut self, node: &SelectionSet, ctx: &mut ValidationContext) {
        let mut finder = FindOverlappingFieldsThatCanBeMerged {
            discoverd_fields: HashMap::new(),
            ctx,
        };

        finder.find_in_selection_set(&node);
    }
}

impl ValidationRule for OverlappingFieldsCanBeMerged {
    fn validate(&mut self, ctx: &mut ValidationContext) {
        self.visit_document(&ctx.operation.clone(), ctx)
    }
}

#[test]
fn detect_trivial_fields() {
    use graphql_parser::query::parse_query;
    use graphql_parser::schema::parse_schema;

    let schema_ast = parse_schema::<String>(
        r#"
        type Query {
          field1: String!
        }
"#,
    )
    .expect("failed to parse schema");

    let query_ast = parse_query::<String>(
        r#"
        query test {
          field1
          field1
        }
  "#,
    )
    .expect("failed to parse query");

    let mut test_context = ValidationContext {
        operation: query_ast,
        schema: schema_ast,
        fragments: HashMap::new(),
        validation_errors: vec![],
    };

    let mut rule = OverlappingFieldsCanBeMerged;

    rule.validate(&mut test_context);

    let messages = test_context
        .validation_errors
        .iter()
        .map(|m| &m.message)
        .collect::<Vec<&String>>();

    assert_eq!(messages, vec![
      "Fields conflict because \"field1\" and \"field1\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
    ]);
}
