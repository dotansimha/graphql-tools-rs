// use graphql_parser::query::Selection;

use super::ValidationRule;
use crate::static_graphql::query::{Document, FragmentDefinition, FragmentSpread, Selection};
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};
use std::collections::HashMap;

/// No fragment cycles
///
/// The graph of fragment spreads must not form any cycles including spreading itself.
/// Otherwise an operation could infinitely spread or infinitely execute on cycles in the underlying data.
///
/// https://spec.graphql.org/draft/#sec-Fragment-spreads-must-not-form-cycles
pub struct NoFragmentsCycle;

struct NoFragmentsCycleHelper<'a> {
	/// Tracks already visited fragments to maintain O(N) and to ensure that cycles
	/// are not redundantly reported.
	visited_fragments: HashMap<String, bool>,
	/// Array of AST nodes used to produce meaningful errors
	fragment_spreads: Vec<FragmentSpread>,
	/// Position in the spread path
	spread_path_index_by_name: HashMap<String, Option<usize>>,
	validation_context: &'a ValidationContext<'a>,
	errors_context: ValidationErrorContext<'a>,
}

impl<'a> QueryVisitor<NoFragmentsCycleHelper<'a>> for NoFragmentsCycle {
	fn leave_document(
		&self,
		_node: &Document,
		_visitor_context: &mut NoFragmentsCycleHelper<'a>,
	) {
		_visitor_context
			.validation_context
			.fragments
			.iter()
			.for_each(|(fragment_name, fragment)| {
				if !_visitor_context
					.visited_fragments
					.contains_key(fragment_name)
				{
					detect_cycles(fragment.clone(), _visitor_context);
				}
			});
	}
}

/// This does a straight-forward DFS to find cycles.
/// It does not terminate when a cycle was found but continues to explore
/// the graph to find all possible cycles.
fn detect_cycles(fragment: FragmentDefinition, ctx: &mut NoFragmentsCycleHelper) {
	if ctx.visited_fragments.contains_key(fragment.name.as_str()) {
		return;
	}

	// mark fragment as visited
	ctx.visited_fragments.insert(fragment.name.clone(), true);

	// get all the fragment spreads for the current fragment
	let spreads: Vec<FragmentSpread> = fragment
		.selection_set
		.items
		.iter()
		.map(|f| {
			if let Selection::FragmentSpread(fragment_spread) = f {
				Some(fragment_spread.clone())
			} else {
				None
			}
		})
		.filter(|x| x.is_some())
		.map(|x| x.unwrap())
		.collect::<Vec<FragmentSpread>>();

	if spreads.len() == 0 {
		return;
	}

	ctx.spread_path_index_by_name
		.insert(fragment.name.clone(), Some(spreads.len()));

	for spread_node in spreads {
		let spread_name = spread_node.fragment_name.clone();
		let cycle_index = ctx.spread_path_index_by_name.get(spread_name.as_str());
		ctx.fragment_spreads.push(spread_node.clone());

		match cycle_index {
			Some(index) => match index {
				Some(index) => {
					let cycle_path = &ctx.fragment_spreads[0..index.clone()];
					ctx.errors_context.report_error(ValidationError {
						locations: cycle_path
							.iter()
							.map(|f| f.position.clone())
							.collect(),
						message: format!(
									"Cannot spread fragment \"{}\" within itself.",
									spread_name
								),
					})
				}
				None => {
					let fragment_spread = ctx
						.validation_context
						.fragments
						.get(spread_name.as_str());
					if fragment_spread.is_some() {
						detect_cycles(
							fragment_spread.unwrap().clone(),
							ctx,
						);
					}
				}
			},
			None => {
				let fragment_spread =
					ctx.validation_context.fragments.get(spread_name.as_str());
				if fragment_spread.is_some() {
					detect_cycles(fragment_spread.unwrap().clone(), ctx);
				}
			}
		}

		ctx.fragment_spreads.pop();
	}
	ctx.spread_path_index_by_name
		.insert(fragment.name.clone(), None);
}

impl<'a> NoFragmentsCycleHelper<'a> {
	fn new(validation_context: &'a ValidationContext<'a>) -> Self {
		NoFragmentsCycleHelper {
			visited_fragments: HashMap::new(),
			spread_path_index_by_name: HashMap::new(),
			fragment_spreads: Vec::new(),
			validation_context: validation_context,
			errors_context: ValidationErrorContext::new(validation_context),
		}
	}
}

impl ValidationRule for NoFragmentsCycle {
	fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
		let operation = ctx.operation.clone();
		let mut helper = NoFragmentsCycleHelper::new(ctx);
		self.visit_document(&operation, &mut helper);
		return helper.errors_context.errors;
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
fn no_spreading_indirectly_within_inline_fragment() {
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
}
