use crate::validation::utils::{ValidationContext, ValidationError};

pub trait ValidationRule<'a>: Send + Sync {
    fn validate(&self, _ctx: &'a ValidationContext<'a>) -> Vec<ValidationError> {
        unimplemented!("Missing ValidationRule:validate implementation");
    }
}
