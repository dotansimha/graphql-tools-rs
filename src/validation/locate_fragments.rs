use std::collections::HashMap;

use crate::{ast::QueryVisitor, static_graphql::query};

pub struct LocateFragments {
    located_fragments: HashMap<String, crate::static_graphql::query::FragmentDefinition>,
}

impl QueryVisitor<'_, LocateFragments> for LocateFragments {
    fn enter_fragment_definition(
        &self,
        _node: &query::FragmentDefinition,
        _ctx: &mut LocateFragments,
    ) {
        let clone = _node.clone().to_owned();
        _ctx.located_fragments.insert(_node.name.clone(), clone);
    }
}

impl LocateFragments {
    pub fn new() -> Self {
        Self {
            located_fragments: HashMap::new(),
        }
    }

    pub fn locate_fragments(
        &mut self,
        operation: &query::Document,
    ) -> HashMap<String, query::FragmentDefinition> {
        let mut visitor = LocateFragments {
            located_fragments: HashMap::new(),
        };

        self.visit_document(operation, &mut visitor);

        visitor.located_fragments
    }
}
