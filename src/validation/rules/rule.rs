use crate::validation::utils::{ValidationContext, ValidationError};

pub trait ValidationRule: Send + Sync {
    fn validate<'a>(&self, _ctx: &'a ValidationContext<'a>) -> Vec<ValidationError> {
        unimplemented!("Missing ValidationRule:validate implementation");
    }
}
