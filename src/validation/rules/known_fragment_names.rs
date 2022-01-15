use super::ValidationRule;
use crate::ast::{visit_document, OperationVisitor, OperationVisitorContext};
use crate::static_graphql::query::*;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// Known fragment names
///
/// A GraphQL document is only valid if all `...Fragment` fragment spreads refer
/// to fragments defined in the same document.
///
/// See https://spec.graphql.org/draft/#sec-Fragment-spread-target-defined
pub struct KnownFragmentNames;

impl<'a> OperationVisitor<'a, ValidationErrorContext> for KnownFragmentNames {
    fn enter_fragment_spread(
        &mut self,
        visitor_context: &mut crate::ast::OperationVisitorContext<ValidationErrorContext>,
        fragment_spread: &FragmentSpread,
    ) {
        match visitor_context
            .known_fragments
            .get(&fragment_spread.fragment_name)
        {
            None => visitor_context.user_context.report_error(ValidationError {
                locations: vec![fragment_spread.position],
                message: format!("Unknown fragment \"{}\".", fragment_spread.fragment_name),
            }),
            _ => {}
        }
    }
}

impl ValidationRule for KnownFragmentNames {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = ValidationErrorContext::new();

        visit_document(
            &mut KnownFragmentNames {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut helper, &ctx.operation, &ctx.schema),
        );

        helper.errors
    }
}

#[test]
fn valid_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownFragmentNames {}));
    let errors = test_operation_with_schema(
        "{
          human(id: 4) {
            ...HumanFields1
            ... on Human {
              ...HumanFields2
            }
            ... {
              name
            }
          }
        }
        fragment HumanFields1 on Human {
          name
          ...HumanFields3
        }
        fragment HumanFields2 on Human {
          name
        }
        fragment HumanFields3 on Human {
          name
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn invalid_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownFragmentNames {}));
    let errors = test_operation_with_schema(
        "{
          human(id: 4) {
            ...UnknownFragment1
            ... on Human {
              ...UnknownFragment2
            }
          }
        }
        fragment HumanFields on Human {
          name
          ...UnknownFragment3
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages,
        vec![
            "Unknown fragment \"UnknownFragment1\".",
            "Unknown fragment \"UnknownFragment2\".",
            "Unknown fragment \"UnknownFragment3\".",
        ]
    );
}
