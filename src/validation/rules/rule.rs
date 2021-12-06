pub struct ValidationContext<'a> {
  pub operation: &'a graphql_parser::query::Document<'a, String>,
  pub schema: &'a graphql_parser::schema::Document<'a, String>,
}

pub struct ValidationError {}

pub trait ValidationRule<'a> {
  fn validate(&mut self, _ctx: &ValidationContext<'a>) {
    unimplemented!("Missing ValidationRule:validate")
  }
}