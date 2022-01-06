use super::ValidationRule;
use crate::static_graphql::query::{FragmentDefinition, FragmentSpread};
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{
    ast::{
        ext::{AstNodeWithName, FragmentSpreadExtraction},
        QueryVisitor,
    },
    validation::utils::ValidationContext,
};
use std::collections::HashMap;

/// No fragment cycles
///
/// The graph of fragment spreads must not form any cycles including spreading itself.
/// Otherwise an operation could infinitely spread or infinitely execute on cycles in the underlying data.
///
/// https://spec.graphql.org/draft/#sec-Fragment-spreads-must-not-form-cycles
pub struct NoFragmentsCycle;

struct NoFragmentsCycleHelper<'a> {
    visited_fragments: HashMap<String, bool>,

    errors_context: ValidationErrorContext<'a>,
}

impl<'a> QueryVisitor<NoFragmentsCycleHelper<'a>> for NoFragmentsCycle {
    fn enter_fragment_definition(
        &self,
        fragment: &FragmentDefinition,
        visitor_context: &mut NoFragmentsCycleHelper<'a>,
    ) {
        let mut spread_paths: Vec<FragmentSpread> = vec![];
        let mut spread_path_index_by_name: HashMap<String, usize> = HashMap::new();
        detect_cycles(
            fragment,
            &mut spread_paths,
            &mut spread_path_index_by_name,
            visitor_context,
        );
    }
}

/// This does a straight-forward DFS to find cycles.
/// It does not terminate when a cycle was found but continues to explore
/// the graph to find all possible cycles.
fn detect_cycles(
    fragment: &FragmentDefinition,
    spread_paths: &mut Vec<FragmentSpread>,
    spread_path_index_by_name: &mut HashMap<String, usize>,
    ctx: &mut NoFragmentsCycleHelper,
) {
    if ctx.visited_fragments.contains_key(&fragment.name) {
        return;
    }

    ctx.visited_fragments.insert(fragment.name.clone(), true);

    let spread_nodes = fragment.selection_set.get_recursive_fragment_spreads();

    if spread_nodes.len() == 0 {
        return;
    }

    spread_path_index_by_name.insert(fragment.name.clone(), spread_paths.len());

    for spread_node in spread_nodes {
        let spread_name = spread_node.fragment_name.clone();
        spread_paths.push(spread_node);

        match spread_path_index_by_name.get(&spread_name) {
            None => {
                if let Some(spread_def) = ctx.errors_context.ctx.fragments.get(&spread_name) {
                    detect_cycles(spread_def, spread_paths, spread_path_index_by_name, ctx);
                }
            }
            Some(cycle_index) => {
                let cycle_path = &spread_paths[cycle_index.clone()..];
                let via_path = match cycle_path.len() {
                    0 => vec![],
                    _ => cycle_path[0..cycle_path.len() - 1]
                        .iter()
                        .map(|s| format!("\"{}\"", s.node_name().unwrap()))
                        .collect::<Vec<String>>(),
                };

                ctx.errors_context.report_error(ValidationError {
                    locations: cycle_path.iter().map(|f| f.position.clone()).collect(),
                    message: match via_path.len() {
                        0 => format!("Cannot spread fragment \"{}\" within itself.", spread_name),
                        _ => format!(
                            "Cannot spread fragment \"{}\" within itself via {}.",
                            spread_name,
                            via_path.join(", ")
                        ),
                    },
                })
            }
        }

        spread_paths.pop();
    }

    spread_path_index_by_name.remove(&fragment.name);
}

impl<'a> NoFragmentsCycleHelper<'a> {
    fn new(validation_context: &'a ValidationContext<'a>) -> Self {
        NoFragmentsCycleHelper {
            visited_fragments: HashMap::new(),
            errors_context: ValidationErrorContext::new(validation_context),
        }
    }
}

impl ValidationRule for NoFragmentsCycle {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = NoFragmentsCycleHelper::new(ctx);
        self.visit_document(&ctx.operation, &mut helper);

        helper.errors_context.errors
    }
}

#[test]
fn single_reference_is_valid() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Dog { ...fragB }
		fragment fragB on Dog { name }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors).len();
    assert_eq!(mes, 0);
}

#[test]
fn spreading_twice_is_not_circular() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Dog { ...fragB, ...fragB }
		fragment fragB on Dog { name }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors).len();
    assert_eq!(mes, 0);
}

#[test]
fn spreading_twice_indirectly_is_not_circular() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Dog { ...fragB, ...fragC }
		fragment fragB on Dog { ...fragC }
		fragment fragC on Dog { name }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors).len();
    assert_eq!(mes, 0);
}

#[test]
fn double_spread_within_abstract_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment nameFragment on Pet {
			... on Dog { name }
			... on Cat { name }
		      }
		
		      fragment spreadsInAnon on Pet {
			... on Dog { ...nameFragment }
			... on Cat { ...nameFragment }
		      }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors).len();
    assert_eq!(mes, 0);
}

#[test]
fn does_not_false_positive_on_unknown_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment nameFragment on Pet {
			...UnknownFragment
		      }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors).len();
    assert_eq!(mes, 0);
}

#[test]
fn spreading_recursively_within_field_fails() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Human { relatives { ...fragA } }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 1);
    assert_eq!(mes, vec!["Cannot spread fragment \"fragA\" within itself."]);
}

#[test]
fn no_spreading_itself_directly() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "
        fragment fragA on Dog { ...fragA }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 1);
    assert_eq!(mes, vec!["Cannot spread fragment \"fragA\" within itself."]);
}

#[test]
fn no_spreading_itself_directly_within_inline_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Pet {
			... on Dog {
			  ...fragA
			}
		      }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 1);
    assert_eq!(mes, vec!["Cannot spread fragment \"fragA\" within itself."]);
}

#[test]
fn no_spreading_itself_indirectly() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Dog { ...fragB }
		fragment fragB on Dog { ...fragA }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 1);
    assert_eq!(
        mes,
        vec!["Cannot spread fragment \"fragA\" within itself via \"fragB\"."]
    );
}

#[test]
fn no_spreading_itself_indirectly_reports_opposite_order() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragB on Dog { ...fragA }
		fragment fragA on Dog { ...fragB }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 1);
    assert_eq!(
        mes,
        vec!["Cannot spread fragment \"fragB\" within itself via \"fragA\"."]
    );
}

#[test]
fn no_spreading_itself_indirectly_within_inline_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Pet {
			... on Dog {
			  ...fragB
			}
		      }
		      fragment fragB on Pet {
			... on Dog {
			  ...fragA
			}
		      }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 1);
    assert_eq!(
        mes,
        vec!["Cannot spread fragment \"fragA\" within itself via \"fragB\"."]
    );
}

#[test]
fn no_spreading_itself_deeply() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Dog { ...fragB }
        fragment fragB on Dog { ...fragC }
        fragment fragC on Dog { ...fragO }
        fragment fragX on Dog { ...fragY }
        fragment fragY on Dog { ...fragZ }
        fragment fragZ on Dog { ...fragO }
        fragment fragO on Dog { ...fragP }
        fragment fragP on Dog { ...fragA, ...fragX }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 2);
    assert_eq!(
        mes,
        vec![
            "Cannot spread fragment \"fragA\" within itself via \"fragB\", \"fragC\", \"fragO\", \"fragP\".",
            "Cannot spread fragment \"fragO\" within itself via \"fragP\", \"fragX\", \"fragY\", \"fragZ\".",
        ]
    );
}

#[test]
fn no_spreading_itself_deeply_two_paths() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "fragment fragA on Dog { ...fragB, ...fragC }
	fragment fragB on Dog { ...fragA }
	fragment fragC on Dog { ...fragA }",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 2);
    assert_eq!(
        mes,
        vec![
            "Cannot spread fragment \"fragA\" within itself via \"fragB\".",
            "Cannot spread fragment \"fragA\" within itself via \"fragC\".",
        ]
    );
}

#[test]
fn no_spreading_itself_deeply_two_paths_alt_traverse_order() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "
        fragment fragA on Dog { ...fragC }
        fragment fragB on Dog { ...fragC }
        fragment fragC on Dog { ...fragA, ...fragB }
        ",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 2);
    assert_eq!(
        mes,
        vec![
            "Cannot spread fragment \"fragA\" within itself via \"fragC\".",
            "Cannot spread fragment \"fragC\" within itself via \"fragB\".",
        ]
    );
}

#[test]
fn no_spreading_itself_deeply_and_immediately() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(NoFragmentsCycle {}));
    let errors = test_operation_with_schema(
        "
          fragment fragA on Dog { ...fragB }
		      fragment fragB on Dog { ...fragB, ...fragC }
		      fragment fragC on Dog { ...fragA, ...fragB }
        ",
        TEST_SCHEMA,
        &mut plan,
    );

    let mes = get_messages(&errors);
    assert_eq!(mes.len(), 3);
    assert_eq!(
        mes,
        vec![
            "Cannot spread fragment \"fragB\" within itself.",
            "Cannot spread fragment \"fragA\" within itself via \"fragB\", \"fragC\".",
            "Cannot spread fragment \"fragB\" within itself via \"fragC\".",
        ]
    );
}
