use super::rules::OverlappingFieldsCanBeMerged;
use super::rules::ValidationRule;
use super::utils::ValidationError;
use super::validate::validate;
use super::validate::ValidationPlan;

#[cfg(test)]
pub fn create_default_ruleset_plan<'a>() -> ValidationPlan<'a> {
    let mut plan = ValidationPlan { rules: Vec::new() };
    plan.add_rule(Box::new(OverlappingFieldsCanBeMerged {}));

    plan
}

#[cfg(test)]
pub static TEST_SCHEMA: &str = "
scalar Boolean
scalar Float
scalar Int
scalar ID
scalar String

interface Mammal {
  mother: Mammal
  father: Mammal
}
interface Pet {
  name(surname: Boolean): String
}
interface Canine implements Mammal {
  name(surname: Boolean): String
  mother: Canine
  father: Canine
}
enum DogCommand {
  SIT
  HEEL
  DOWN
}
type Dog implements Pet & Mammal & Canine {
  name(surname: Boolean): String
  nickname: String
  barkVolume: Int
  barks: Boolean
  doesKnowCommand(dogCommand: DogCommand): Boolean
  isHouseTrained(atOtherHomes: Boolean = true): Boolean
  isAtLocation(x: Int, y: Int): Boolean
  mother: Dog
  father: Dog
}
type Cat implements Pet {
  name(surname: Boolean): String
  nickname: String
  meows: Boolean
  meowsVolume: Int
  furColor: FurColor
}
union CatOrDog = Cat | Dog
type Human {
  name(surname: Boolean): String
  pets: [Pet]
  relatives: [Human]
}
enum FurColor {
  BROWN
  BLACK
  TAN
  SPOTTED
  NO_FUR
  UNKNOWN
}
input ComplexInput {
  requiredField: Boolean!
  nonNullField: Boolean! = false
  intField: Int
  stringField: String
  booleanField: Boolean
  stringListField: [String]
}
type ComplicatedArgs {
  # TODO List
  # TODO Coercion
  # TODO NotNulls
  intArgField(intArg: Int): String
  nonNullIntArgField(nonNullIntArg: Int!): String
  stringArgField(stringArg: String): String
  booleanArgField(booleanArg: Boolean): String
  enumArgField(enumArg: FurColor): String
  floatArgField(floatArg: Float): String
  idArgField(idArg: ID): String
  stringListArgField(stringListArg: [String]): String
  stringListNonNullArgField(stringListNonNullArg: [String!]): String
  complexArgField(complexArg: ComplexInput): String
  multipleReqs(req1: Int!, req2: Int!): String
  nonNullFieldWithDefault(arg: Int! = 0): String
  multipleOpts(opt1: Int = 0, opt2: Int = 0): String
  multipleOptAndReq(req1: Int!, req2: Int!, opt1: Int = 0, opt2: Int = 0): String
}
type QueryRoot {
  human(id: ID): Human
  dog: Dog
  cat: Cat
  pet: Pet
  catOrDog: CatOrDog
  complicatedArgs: ComplicatedArgs
}
schema {
  query: QueryRoot
}
directive @onField on FIELD";

#[cfg(test)]
pub fn create_plan_from_rule<'a>(rule: Box<dyn ValidationRule<'a>>) -> ValidationPlan<'a> {
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
    operation: &'a str,
    plan: &'a mut ValidationPlan<'a>,
) -> Vec<ValidationError> {
    let schema_ast = graphql_parser::parse_schema(
        "
type Query {
  dummy: String
}
",
    )
    .expect("Failed to parse schema");

    let operation_ast = graphql_parser::parse_query(operation)
        .unwrap()
        .into_static();

    validate(&schema_ast, &operation_ast, &plan)
}

#[cfg(test)]
pub fn test_operation_with_schema<'a>(
    operation: &'static str,
    schema: &'static str,
    plan: &'a mut ValidationPlan<'a>,
) -> Vec<ValidationError> {
    let schema_ast = graphql_parser::parse_schema(schema).expect("Failed to parse schema");

    let operation_ast = graphql_parser::parse_query(operation)
        .unwrap()
        .into_static();

    validate(&schema_ast, &operation_ast, &plan)
}
