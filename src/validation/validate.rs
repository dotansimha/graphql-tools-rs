use super::{
    locate_fragments::LocateFragments,
    rules::ValidationRule,
    utils::{ValidationContext, ValidationError},
};

use crate::{
    ast::TypeInfoRegistry,
    static_graphql::{query, schema},
};

pub struct ValidationPlan {
    pub rules: Vec<Box<dyn ValidationRule>>,
}

impl ValidationPlan {
    pub fn new() -> Self {
        Self { rules: vec![] }
    }

    pub fn from(rules: Vec<Box<dyn ValidationRule>>) -> Self {
        Self { rules }
    }

    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.push(rule);
    }
}

pub fn validate<'a>(
    schema: &'a schema::Document,
    operation: &'a query::Document,
    validation_plan: &'a ValidationPlan,
) -> Vec<ValidationError> {
    let mut fragments_locator = LocateFragments::new();
    let fragments = fragments_locator.locate_fragments(&operation);

    let type_info_registry = TypeInfoRegistry::new(schema);
    let validation_context = ValidationContext {
        operation: operation.clone(),
        schema: schema.clone(),
        fragments,
        type_info_registry: Some(type_info_registry),
    };

    let validation_errors = validation_plan
        .rules
        .iter()
        .flat_map(|rule| rule.validate(&validation_context))
        .collect::<Vec<_>>();

    validation_errors
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
    ",
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
    ",
        &mut default_plan,
    );

    assert_eq!(errors.len(), 0);
}
