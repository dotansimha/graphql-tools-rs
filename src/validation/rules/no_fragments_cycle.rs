use super::ValidationRule;
use crate::validation::utils::ValidationError;
use crate::{
	ast::QueryVisitor,
	static_graphql::query::{FragmentDefinition, FragmentSpread},
	validation::utils::ValidationContext,
};
use graphql_parser::query::Selection;
use std::collections::HashMap;

/// No fragment cycles
///
/// The graph of fragment spreads must not form any cycles including spreading itself.
/// Otherwise an operation could infinitely spread or infinitely execute on cycles in the underlying data.
///
/// https://spec.graphql.org/draft/#sec-Fragment-spreads-must-not-form-cycles
pub struct NoFragmentsCycle;

struct NoFragmentsCycleHelper<'a> {
	// Tracks already visited fragments to maintain O(N) and to ensure that cycles
	// are not redundantly reported.
	visited_fragments: HashMap<String, bool>,
	// Array of AST nodes used to produce meaningful errors
	spread_path: Vec<FragmentDefinition>,
	// Position in the spread path
	fragment_spreads: Vec<FragmentSpread>,
	spread_path_index_by_name: HashMap<String, usize>,
	validation_context: &'a mut ValidationContext,
}

impl<'a> QueryVisitor<NoFragmentsCycleHelper<'a>> for NoFragmentsCycle {
	fn enter_fragment_spread(
		&self,
		_node: &FragmentSpread,
		_visitor_context: &mut NoFragmentsCycleHelper<'a>,
	) {
		_visitor_context.fragment_spreads.push(_node.clone());
	}
	// fn enter_fragment_definition(
	// 	&self,
	// 	_node: &FragmentDefinition,
	// 	_visitor_context: &mut NoFragmentsCycleHelper<'a>,
	// ) {
	// 	let node = _node.clone();
	// 	detect_cycles(node, _visitor_context);
	// 	false;
	// }
	fn leave_fragment_definition(
		&self,
		_node: &FragmentDefinition,
		_visitor_context: &mut NoFragmentsCycleHelper<'a>,
	) {
		let node = _node.clone();
		detect_cycles(node, _visitor_context);
		false;
	}
}

// FragmentDefinition -> FragmentDefinitionNode
// FragmentSpread -> FragmentSpreadNode
// InlierFragment -> InlineFragmentNode

// This does a straight-forward DFS to find cycles.
// It does not terminate when a cycle was found but continues to explore
// the graph to find all possible cycles.
fn detect_cycles(fragment: FragmentDefinition, ctx: &mut NoFragmentsCycleHelper) {
	if ctx.visited_fragments.contains_key(fragment.name.as_str()) {
		return;
	}

	ctx.visited_fragments.insert(fragment.name.clone(), true);
	// let spreads = get_fragment_spreads(&fragment);
	// get all the fragment spreads in the selection set
	// let spreads: Vec<Selection<String>> = fragment.selection_set.items.iter().collect();

	let spreads = ctx.fragment_spreads.clone();

	if spreads.len() == 0 {
		return;
	}

	ctx.spread_path_index_by_name
		.insert(fragment.name.clone(), ctx.spread_path.len());

	for spread_node in spreads {
		let spread_name = spread_node.fragment_name.clone();
		let cycle_index = ctx.spread_path_index_by_name.get(spread_name.as_str());
		ctx.spread_path.push(fragment.clone());

		if cycle_index == None {
			let fragment_spread =
				ctx.validation_context.fragments.get(spread_name.as_str());
			if fragment_spread.is_some() {
				detect_cycles(fragment_spread.unwrap().clone(), ctx);
			}
		} else {
			ctx.validation_context.report_error(ValidationError {
				locations: [ctx.spread_path[0].position.clone()].to_vec(),
				message: format!(
					"Fragment \"{}\" cannot be spread here as it would produce a cycle",
					spread_name
				),
			})
		}
		ctx.spread_path.pop();
	}
}

impl<'a> NoFragmentsCycleHelper<'a> {
	fn new(validation_context: &'a mut ValidationContext) -> Self {
		NoFragmentsCycleHelper {
			visited_fragments: HashMap::new(),
			spread_path: Vec::new(),
			spread_path_index_by_name: HashMap::new(),
			fragment_spreads: Vec::new(),
			validation_context,
		}
	}
}

impl ValidationRule for NoFragmentsCycle {
	fn validate(&self, ctx: &mut ValidationContext) {
		let operation = ctx.operation.clone();
		let mut helper = NoFragmentsCycleHelper::new(ctx);
		self.visit_document(&operation, &mut helper)
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
