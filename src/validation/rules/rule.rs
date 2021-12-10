use crate::validation::utils::ValidationContext;

pub trait ValidationRule<'a> {
    fn validate(&mut self, _ctx: &'a ValidationContext<'a>) {
        unimplemented!("Missing ValidationRule:validate implementation");
    }
}
