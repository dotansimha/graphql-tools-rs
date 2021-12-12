use crate::static_graphql::{query, schema};
use graphql_parser::Pos;
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug)]
pub struct ValidationContext {
    pub operation: query::Document,
    pub schema: schema::Document,
    pub fragments: HashMap<String, crate::static_graphql::query::FragmentDefinition>,
    pub validation_errors: Vec<ValidationError>,
}

#[derive(Debug)]
pub struct ValidationError {
    pub locations: Vec<Pos>,
    pub message: String,
}
