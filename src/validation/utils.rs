use graphql_parser::Pos;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ValidationErrorContext {
    pub errors: Vec<ValidationError>,
}

impl ValidationErrorContext {
    pub fn new() -> ValidationErrorContext {
        ValidationErrorContext { errors: vec![] }
    }

    pub fn report_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }
}

#[derive(Debug)]
pub struct ValidationError {
    pub locations: Vec<Pos>,
    pub message: String,
}
