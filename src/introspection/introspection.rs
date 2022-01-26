use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionQuery {
    __schema: IntrospectionSchema,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionScalarType {
    name: String,
    description: Option<String>,
    #[serde(rename = "specifiedByURL")]
    specified_by_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionInputValue {
    name: String,
    description: Option<String>,
    #[serde(rename = "defaultValue")]
    default_value: Option<String>,
    #[serde(rename = "isDeprecated")]
    is_deprecated: Option<bool>,
    #[serde(rename = "deprecationReason")]
    deprecation_reason: Option<String>,
    #[serde(rename = "type")]
    type_ref: Option<IntrospectionInputTypeRef>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub struct IntrospectionField {
    name: String,
    description: Option<String>,
    args: Vec<IntrospectionInputValue>,
    #[serde(rename = "isDeprecated")]
    is_deprecated: Option<bool>,
    #[serde(rename = "deprecationReason")]
    deprecation_reason: Option<String>,
    #[serde(rename = "type")]
    type_ref: IntrospectionOutputTypeRef,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionObjectType {
    name: String,
    description: Option<String>,
    fields: Vec<IntrospectionField>,
    interfaces: Vec<IntrospectionNamedTypeRef>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionInterfaceType {
    name: String,
    description: Option<String>,
    fields: Vec<IntrospectionField>,
    interfaces: Option<Vec<IntrospectionNamedTypeRef>>,
    #[serde(rename = "possibleTypes")]
    possible_types: Vec<IntrospectionNamedTypeRef>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionUnionType {
    name: String,
    description: Option<String>,
    #[serde(rename = "possibleTypes")]
    possible_types: Vec<IntrospectionNamedTypeRef>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionEnumValue {
    name: String,
    description: Option<String>,
    #[serde(rename = "isDeprecated")]
    is_deprecated: Option<bool>,
    #[serde(rename = "deprecationReason")]
    deprecation_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionEnumType {
    name: String,
    description: Option<String>,
    #[serde(rename = "enumValues")]
    enum_values: Vec<IntrospectionEnumValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionInputObjectType {
    name: String,
    description: Option<String>,
    #[serde(rename = "inputFields")]
    input_fields: Vec<IntrospectionInputValue>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum IntrospectionType {
    SCALAR(IntrospectionScalarType),
    OBJECT(IntrospectionObjectType),
    INTERFACE(IntrospectionInterfaceType),
    UNION(IntrospectionUnionType),
    ENUM(IntrospectionEnumType),
    INPUT_OBJECT(IntrospectionInputObjectType),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum IntrospectionInputType {
    SCALAR(IntrospectionScalarType),
    ENUM(IntrospectionEnumType),
    INPUT_OBJECT(IntrospectionInputObjectType),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum IntrospectionOutputType {
    SCALAR(IntrospectionScalarType),
    OBJECT(IntrospectionObjectType),
    INTERFACE(IntrospectionInterfaceType),
    UNION(IntrospectionUnionType),
    ENUM(IntrospectionEnumType),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionNamedTypeRef {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum IntrospectionOutputTypeRef {
    SCALAR(IntrospectionNamedTypeRef),
    LIST {
        #[serde(rename = "ofType")]
        of_type: Option<Box<IntrospectionOutputTypeRef>>,
    },
    NON_NULL {
        #[serde(rename = "ofType")]
        of_type: Option<Box<IntrospectionOutputTypeRef>>,
    },
    ENUM(IntrospectionNamedTypeRef),
    INPUT_OBJECT(IntrospectionNamedTypeRef),
    UNION(IntrospectionNamedTypeRef),
    OBJECT(IntrospectionNamedTypeRef),
    INTERFACE(IntrospectionNamedTypeRef),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum IntrospectionInputTypeRef {
    LIST {
        #[serde(rename = "ofType")]
        of_type: Option<Box<IntrospectionOutputTypeRef>>,
    },
    NON_NULL {
        #[serde(rename = "ofType")]
        of_type: Option<Box<IntrospectionOutputTypeRef>>,
    },
    SCALAR(IntrospectionNamedTypeRef),
    ENUM(IntrospectionNamedTypeRef),
    INPUT_OBJECT(IntrospectionNamedTypeRef),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionSchema {
    description: Option<String>,
    #[serde(rename = "queryType")]
    query_type: IntrospectionNamedTypeRef,
    #[serde(rename = "mutationType")]
    mutation_type: Option<IntrospectionNamedTypeRef>,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<IntrospectionNamedTypeRef>,
    types: Vec<IntrospectionType>,
    directives: Vec<IntrospectionDirective>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DirectiveLocation {
    QUERY,
    MUTATION,
    SUBSCRIPTION,
    FIELD,
    FRAGMENT_DEFINITION,
    FRAGMENT_SPREAD,
    INLINE_FRAGMENT,
    VARIABLE_DEFINITION,
    /** Type System Definitions */
    SCHEMA,
    SCALAR,
    OBJECT,
    FIELD_DEFINITION,
    ARGUMENT_DEFINITION,
    INTERFACE,
    UNION,
    ENUM,
    ENUM_VALUE,
    INPUT_OBJECT,
    INPUT_FIELD_DEFINITION,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IntrospectionDirective {
    name: String,
    description: Option<String>,
    #[serde(rename = "isRepeatable")]
    is_repeatable: Option<bool>,
    locations: Vec<DirectiveLocation>,
    args: Vec<IntrospectionInputValue>,
}

pub fn parse_introspection<R>(input: R) -> Result<IntrospectionQuery>
where
    R: io::Read,
{
    serde_json::from_reader::<R, IntrospectionQuery>(input)
}

#[test]
fn test_product_introspection() {
    use std::fs::File;
    let json_file = File::open("./src/introspection/test_files/product_introspection.json")
        .expect("failed to open json file");
    parse_introspection(json_file).expect("failed to parse introspection json");
}

#[test]
fn test_github_introspection() {
    use std::fs::File;
    let json_file = File::open("./src/introspection/test_files/github_introspection.json")
        .expect("failed to open json file");
    parse_introspection(json_file).expect("failed to parse introspection json");
}

#[test]
fn test_shopify_introspection() {
    use std::fs::File;
    let json_file = File::open("./src/introspection/test_files/shopify_introspection.json")
        .expect("failed to open json file");
    parse_introspection(json_file).expect("failed to parse introspection json");
}
