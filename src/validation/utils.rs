use crate::ast::QueryVisitor;

pub struct FragmentRef<'a>(
    String,
    &'a graphql_parser::query::FragmentDefinition<'a, String>,
);

pub struct ValidationContext<'a> {
    pub operation: &'a graphql_parser::query::Document<'a, String>,
    pub schema: &'a graphql_parser::schema::Document<'a, String>,
    pub fragments: Vec<FragmentRef<'a>>,
}

pub struct ValidationError {}

pub struct LocateFragments<'a> {
    pub located_fragments: Vec<FragmentRef<'a>>,
}

impl<'a> QueryVisitor<'a> for LocateFragments<'a> {
    fn enter_fragment_definition(
        &mut self,
        _node: &'a graphql_parser::query::FragmentDefinition<'a, String>,
    ) {
        self.located_fragments
            .push(FragmentRef::<'a>(_node.name.clone(), _node));
    }
}

impl<'a> LocateFragments<'a> {
    pub fn locate_fragments(&mut self, operation: &'a graphql_parser::query::Document<'a, String>) {
        self.__visit_document(operation);
    }
}
