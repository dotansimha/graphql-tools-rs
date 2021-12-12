use std::collections::HashMap;

use super::{
    locate_fragments::LocateFragments,
    rules::ValidationRule,
    utils::{ValidationContext, ValidationError},
};

use crate::static_graphql::{query, schema};

pub struct ValidationPlan {
    pub rules: Vec<Box<dyn ValidationRule>>,
}

impl ValidationPlan {
    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.push(rule);
    }
}

pub fn validate(
    schema: &schema::Document,
    operation: &query::Document,
    validation_plan: &ValidationPlan,
) -> Vec<ValidationError> {
    let mut fragments_locator = LocateFragments::new();
    let fragments = fragments_locator.locate_fragments(&operation);
    let mut validation_context = ValidationContext {
        operation: operation.clone(),
        schema: schema.clone(),
        fragments,
        validation_errors: Vec::new(),
    };

    for rule in &validation_plan.rules {
        rule.validate(&mut validation_context);
    }

    validation_context.validation_errors
}

#[test]
fn test_validate_valid_query() {
    use crate::validation::test_utils::*;

    let mut default_plan = create_default_ruleset_plan();
    let errors = test_operation_without_schema(
        "
    query test {
      foo
    }
    "
        .to_owned(),
        &mut default_plan,
    );

    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_valid_fragment() {
    use crate::validation::test_utils::*;

    let mut default_plan = create_default_ruleset_plan();
    let errors = test_operation_without_schema(
        "
        fragment uniqueFields on Dog {
          name
          nickname
        }
    "
        .to_owned(),
        &mut default_plan,
    );

    assert_eq!(errors.len(), 0);
}
