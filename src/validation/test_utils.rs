use super::rules::OverlappingFieldsCanBeMerged;
use super::rules::ValidationRule;
use super::utils::ValidationError;
use super::validate::validate;
use super::validate::ValidationPlan;

#[cfg(test)]
pub fn create_default_ruleset_plan() -> ValidationPlan {
    let mut plan = ValidationPlan { rules: Vec::new() };
    plan.add_rule(Box::new(OverlappingFieldsCanBeMerged {}));

    plan
}

#[cfg(test)]
pub fn create_plan_from_rule(rule: Box<dyn ValidationRule>) -> ValidationPlan {
    let mut rules = Vec::new();
    rules.push(rule);

    let plan = ValidationPlan { rules };

    plan
}

#[cfg(test)]
pub fn get_messages(validation_errors: &Vec<ValidationError>) -> Vec<&String> {
    validation_errors
        .iter()
        .map(|m| &m.message)
        .collect::<Vec<&String>>()
}

#[cfg(test)]
pub fn test_operation_without_schema<'a>(
    operation: String,
    plan: &mut ValidationPlan,
) -> Vec<ValidationError> {
    let schema_ast = graphql_parser::parse_schema::<String>(
        "
type Query {
  dummy: String
}
",
    )
    .expect("Failed to parse schema");

    let operation_ast = graphql_parser::parse_query(&operation)
        .unwrap()
        .into_static();

    validate(&schema_ast, &operation_ast, &plan)
}
