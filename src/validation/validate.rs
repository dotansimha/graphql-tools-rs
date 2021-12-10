use std::collections::HashMap;

use super::{
    locate_fragments::LocateFragments,
    rules::{OverlappingFieldsCanBeMerged, ValidationRule},
    utils::ValidationContext,
};

use crate::static_graphql::{query, schema};

fn validate(schema: &schema::Document, operation: &query::Document) {
    let mut fragments_locator = LocateFragments {
        located_fragments: HashMap::new(),
    };

    fragments_locator.locate_fragments(&operation);

    let mut validation_context = ValidationContext {
        operation: operation.clone(),
        schema: schema.clone(),
        fragments: fragments_locator.located_fragments,
        validation_errors: Vec::new(),
    };

    let rules = vec![OverlappingFieldsCanBeMerged {}];

    for mut rule in rules {
        rule.validate(&mut validation_context);
    }
}

#[test]
fn test_validate_valid_query() {
    // let schema_ast = graphql_parser::parse_schema::<String>(
    //     "
    // type Query {
    //   foo: String
    // }
    // ",
    // )
    // .expect("Failed to parse schema");

    // let operation_ast = graphql_parser::parse_query::<String>(
    //     "
    // query test {
    //   foo
    // }
    // ",
    // )
    // .expect("Failed to parse query");

    // validate(&schema_ast, &operation_ast);
}
