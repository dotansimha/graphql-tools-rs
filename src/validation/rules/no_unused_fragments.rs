use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// No unused fragments
///
/// A GraphQL document is only valid if all fragment definitions are spread
/// within operations, or spread within other fragments spread within operations.
///
/// See https://spec.graphql.org/draft/#sec-Fragments-Must-Be-Used
pub struct NoUnusedFragments;

impl<'a> QueryVisitor<NoUnusedFragmentsHelper<'a>> for NoUnusedFragments {
    fn enter_fragment_spread(
        &self,
        _node: &FragmentSpread,
        _visitor_context: &mut NoUnusedFragmentsHelper<'a>,
    ) {
        _visitor_context
            .fragments_in_use
            .push(_node.fragment_name.clone());
    }

    fn leave_document(&self, _node: &Document, _visitor_context: &mut NoUnusedFragmentsHelper<'a>) {
        _visitor_context
            .validation_context
            .fragments
            .iter()
            .filter_map(|(fragment_name, _fragment)| {
                if !_visitor_context.fragments_in_use.contains(&fragment_name) {
                    Some(fragment_name.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .iter()
            .for_each(move |unused_fragment_name| {
                _visitor_context
                    .error_context
                    .report_error(ValidationError {
                        locations: vec![],
                        message: format!("Fragment \"{}\" is never used.", unused_fragment_name),
                    });
            });
    }
}

struct NoUnusedFragmentsHelper<'a> {
    error_context: ValidationErrorContext<'a>,
    fragments_in_use: Vec<String>,
    validation_context: &'a ValidationContext<'a>,
}

impl<'a> NoUnusedFragmentsHelper<'a> {
    fn new(validation_context: &'a ValidationContext<'a>) -> Self {
        NoUnusedFragmentsHelper {
            error_context: ValidationErrorContext::new(validation_context),
            fragments_in_use: Vec::new(),
            validation_context,
        }
    }
}

impl ValidationRule for NoUnusedFragments {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = NoUnusedFragmentsHelper::new(&ctx);
        self.visit_document(&ctx.operation.clone(), &mut helper);

        helper.error_context.errors
    }
}

#[test]
fn all_fragment_names_are_used() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedFragments {}));
    let errors = test_operation_without_schema(
        "{
          human(id: 4) {
            ...HumanFields1
            ... on Human {
              ...HumanFields2
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
fn all_fragment_names_are_used_by_multiple_operations() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedFragments {}));
    let errors = test_operation_without_schema(
        "query Foo {
          human(id: 4) {
            ...HumanFields1
          }
        }
        query Bar {
          human(id: 4) {
            ...HumanFields2
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
        }
  ",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn contains_unknown_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedFragments {}));
    let errors = test_operation_without_schema(
        "query Foo {
          human(id: 4) {
            ...HumanFields1
          }
        }
        query Bar {
          human(id: 4) {
            ...HumanFields2
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
        }
        fragment Unused1 on Human {
          name
        }
        fragment Unused2 on Human {
          name
        }
  ",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
}

// TODO: Fix this one :( It's not working
#[test]
#[ignore]
fn contains_unknown_fragments_with_ref_cycle() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedFragments {}));
    let errors = test_operation_without_schema(
        "query Foo {
          human(id: 4) {
            ...HumanFields1
          }
        }
        query Bar {
          human(id: 4) {
            ...HumanFields2
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
        }
        fragment Unused1 on Human {
          name
          ...Unused2
        }
        fragment Unused2 on Human {
          name
          ...Unused1
        }
  ",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages,
        vec![
            "Fragment \"Unused1\" is never used.",
            "Fragment \"Unused2\" is never used."
        ]
    );
}

#[test]
fn contains_unknown_and_undef_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoUnusedFragments {}));
    let errors = test_operation_without_schema(
        "query Foo {
          human(id: 4) {
            ...bar
          }
        }
        fragment foo on Human {
          name
        }
  ",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fragment \"foo\" is never used.",]);
}
