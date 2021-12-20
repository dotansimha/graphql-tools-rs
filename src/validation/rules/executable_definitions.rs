use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationError;
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

/// Executable definitions
///
/// A GraphQL document is only valid for execution if all definitions are either
/// operation or fragment definitions.
///
///
/// See https://spec.graphql.org/draft/#sec-Executable-Definitions
pub struct ExecutableDefinitions;

impl QueryVisitor<ValidationContext> for ExecutableDefinitions {
	fn enter_document(&self, _node: &Document, visitor_context: &mut ValidationContext) {
		for _definition in &_node.definitions {
			let definition = _definition.to_string();

			fn is_executable_definition(node: &Definition) -> bool {
				match node {
					Definition::Operation(_) => true,
					Definition::Fragment(_) => true,
				}
			}

			if is_executable_definition(_definition) == false {
				let def_name = if definition == "SchemaDefinition"
					|| definition == "SchemaExtensions"
				{
					"schema".to_string()
				} else {
					format!("\" {} \"", definition).to_string()
				};

				print!("{}", def_name);

				visitor_context.report_error(ValidationError {
					message: format!(
						"The {} definition is not executable.",
						def_name
					),
					locations: vec![],
				});
			}
		}
		false;
	}
}

impl ValidationRule for ExecutableDefinitions {
	fn validate(&self, ctx: &mut ValidationContext) -> () {
		self.visit_document(&ctx.operation.clone(), ctx)
	}
}

#[test]
fn only_operation() {
	use crate::validation::test_utils::*;
	let mut plan = create_plan_from_rule(Box::new(ExecutableDefinitions {}));
	let errors = test_operation_with_schema(
		"query Foo {
	      dog {
	        name
	      }
	    }",
		TEST_SCHEMA,
		&mut plan,
	);
	assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn with_operation_and_fragment() {
	use crate::validation::test_utils::*;
	let mut plan = create_plan_from_rule(Box::new(ExecutableDefinitions {}));
	let errors = test_operation_with_schema(
		"query Foo {
	      dog {
	        name
		...Frag
	      }

	      fragment Frag on Dog {
		name
	      }
	    }",
		TEST_SCHEMA,
		&mut plan,
	);
	assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn with_type_definition() {
	use crate::validation::test_utils::*;
	let mut plan = create_plan_from_rule(Box::new(ExecutableDefinitions {}));
	let errors = test_operation_with_schema(
		"
		query Foo {
		  dog {
	  		name
		}
		type Cow {
			name: String
		}
		extend type Dog {
		      color: String
		}
		}",
		TEST_SCHEMA,
		&mut plan,
	);
	let messages = get_messages(&errors);
	print!("{:?}", messages);
	assert_eq!(messages.len(), 2);
}
