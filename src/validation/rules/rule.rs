use crate::validation::utils::ValidationContext;

pub trait ValidationRule: Send {
    fn validate(&self, _ctx: &mut ValidationContext) {
        unimplemented!("Missing ValidationRule:validate implementation");
    }
}

