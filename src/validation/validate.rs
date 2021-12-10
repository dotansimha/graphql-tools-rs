use std::collections::HashMap;

use super::{
    rules::{OverlappingFieldsCanBeMerged, ValidationRule},
    utils::{LocateFragments, ValidationContext},
};

fn validate<'a>(
    schema: &'a graphql_parser::schema::Document<'a, String>,
    operation: &'a graphql_parser::query::Document<'a, String>,
) {
    // let mut fragments_locator = LocateFragments {
    //     located_fragments: HashMap::new(),
    // };

    // fragments_locator.locate_fragments(&operation);

    let validation_context = ValidationContext::new(&operation, &schema);

    // let rules = vec![OverlappingFieldsCanBeMerged {}];

    let mut a = OverlappingFieldsCanBeMerged {};

    a.validate(&validation_context);

    // for mut rule in rules {
    //     rule.validate(&validation_context);
    // }
}

#[test]
fn test_validate_valid_query() {
    let schema_ast = graphql_parser::parse_schema::<String>(
        "
    type Query {
      foo: String
    }
    ",
    )
    .expect("Failed to parse schema");

    let operation_ast = graphql_parser::parse_query::<String>(
        "
    query test {
      foo
    }
    ",
    )
    .expect("Failed to parse query");

    validate(&schema_ast, &operation_ast);
}
