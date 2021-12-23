use graphql_parser::query::Selection;

use super::ValidationRule;
use crate::{
	ast::QueryVisitor,
	static_graphql::query::{FragmentDefinition, FragmentSpread, SelectionSet},
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
	// Tracks already visited fragments to maintain O(N) and to ensure that cycles
	// are not redundantly reported.
	visited_fragments: HashMap<String, bool>,
	// Array of AST nodes used to produce meaningful errors
	spread_path: Vec<FragmentDefinition>,
	// Position in the spread path
	spread_path_index_by_name: HashMap<String, usize>,
	validation_context: &'a mut ValidationContext,
}

impl<'a> QueryVisitor<NoFragmentsCycleHelper<'a>> for NoFragmentsCycle {
	fn enter_fragment_definition(
		&self,
		_node: &FragmentDefinition,
		_visitor_context: &mut NoFragmentsCycleHelper<'a>,
	) {
		let node = _node.clone();
		detect_cycles(node, _visitor_context)
	}
}

fn get_fragment_spreads(fragment: &FragmentDefinition) -> Vec<FragmentSpread> {
	return fragment
		.selection_set
		.items
		.iter()
		.filter_map(|selection| match selection {
			Selection::FragmentSpread(fragment_spread) => Some(fragment_spread.clone()),
			_ => None,
		})
		.collect();
}

// FragmentDefinition -> FragmentDefinitionNode
// FragmentSpread -> FragmentSpreadNode
// InlierFragment -> InlineFragmentNode

// getFragmentSpreads(
// 	node: SelectionSetNode,
//       ): ReadonlyArray<FragmentSpreadNode> {
// 	let spreads = this._fragmentSpreads.get(node);
// 	if (!spreads) {
// 	  spreads = [];
// 	  const setsToVisit: Array<SelectionSetNode> = [node];
// 	  let set: SelectionSetNode | undefined;
// 	  while ((set = setsToVisit.pop())) {
// 	    for (const selection of set.selections) {
// 	      if (selection.kind === Kind.FRAGMENT_SPREAD) {
// 		spreads.push(selection);
// 	      } else if (selection.selectionSet) {
// 		setsToVisit.push(selection.selectionSet);
// 	      }
// 	    }
// 	  }
// 	  this._fragmentSpreads.set(node, spreads);
// 	}
// 	return spreads;
//       }

// This does a straight-forward DFS to find cycles.
// It does not terminate when a cycle was found but continues to explore
// the graph to find all possible cycles.
fn detect_cycles(fragment: FragmentDefinition, ctx: &mut NoFragmentsCycleHelper) {
	if ctx.visited_fragments.contains_key(fragment.name.as_str()) {
		return;
	}

	ctx.visited_fragments.insert(fragment.name.clone(), true);

	let spread_nodes = get_fragment_spreads(&fragment);
	if spread_nodes.len() == 0 {
		return;
	}

	ctx.spread_path_index_by_name
		.insert(fragment.name.clone(), ctx.spread_path.len());

	for spread_node in spread_nodes {
		let spread_name = spread_node.fragment_name.as_str();
		let cycle_index = ctx.spread_path_index_by_name.get(spread_name);

		ctx.spread_path.push(fragment.clone());
	}
}

impl<'a> NoFragmentsCycleHelper<'a> {
	fn new(validation_context: &'a mut ValidationContext) -> Self {
		NoFragmentsCycleHelper {
			visited_fragments: HashMap::new(),
			spread_path: Vec::new(),
			spread_path_index_by_name: HashMap::new(),
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

	assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn spreading_indirectly_within_inline_fragment() {
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

	assert_eq!(get_messages(&errors).len(), 1);
}
