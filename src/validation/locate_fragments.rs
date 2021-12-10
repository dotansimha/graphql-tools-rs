use std::collections::HashMap;

use crate::{
    ast::{DefaultVisitorContext, QueryVisitor},
    static_graphql::query,
};

pub struct LocateFragments {
    pub located_fragments: HashMap<String, crate::static_graphql::query::FragmentDefinition>,
}

impl QueryVisitor for LocateFragments {
    fn enter_fragment_definition(
        &mut self,
        _node: &query::FragmentDefinition,
        _ctx: &mut DefaultVisitorContext,
    ) {
        let clone = _node.clone().to_owned();
        self.located_fragments.insert(_node.name.clone(), clone);
    }
}

impl LocateFragments {
    pub fn locate_fragments(&mut self, operation: &query::Document) {
        self.visit_document(operation, &mut DefaultVisitorContext {});
    }
}
