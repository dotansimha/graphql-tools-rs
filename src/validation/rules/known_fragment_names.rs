use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// Known fragment names
///
/// A GraphQL document is only valid if all `...Fragment` fragment spreads refer
/// to fragments defined in the same document.
///
/// See https://spec.graphql.org/draft/#sec-Fragment-spread-target-defined
pub struct KnownFragmentNamesRule;

impl<'a> QueryVisitor<'a, ValidationErrorContext<'a>> for KnownFragmentNamesRule {
    fn enter_fragment_spread(
        &self,
        _node: &FragmentSpread,
        _visitor_context: &mut ValidationErrorContext,
    ) {
        let fragment_def = _visitor_context.ctx.fragments.get(&_node.fragment_name);

        match fragment_def {
            None => _visitor_context.report_error(ValidationError {
                locations: vec![_node.position],
                message: format!("Unknown fragment \"{}\".", _node.fragment_name),
            }),
            _ => {}
        }
    }
}

impl ValidationRule for KnownFragmentNamesRule {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut error_context = ValidationErrorContext::new(ctx);
        self.visit_document(&ctx.operation.clone(), &mut error_context);

        error_context.errors
    }
}

#[test]
fn valid_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownFragmentNamesRule {}));
    let errors = test_operation_without_schema(
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
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn invalid_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(KnownFragmentNamesRule {}));
    let errors = test_operation_without_schema(
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
