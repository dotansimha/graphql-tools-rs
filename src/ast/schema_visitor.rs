use graphql_parser::schema::{Document, InputValue, Definition, SchemaDefinition, DirectiveDefinition, TypeDefinition, ObjectType, ScalarType, EnumType, Field, EnumValue, UnionType, InputObjectType, InterfaceType};

pub trait SchemaVisitor<'a> {
  fn visit_schema_document(&mut self, document: &'a Document<String>) {
    self.enter_document(document);

    for definition in &document.definitions {
      match definition {
        Definition::SchemaDefinition(schema_definition) => {
          self.enter_schema_definition(schema_definition);
          self.leave_schema_definition(schema_definition);
        }
        Definition::TypeDefinition(type_definition) => {
          self.enter_type_definition(type_definition);
          
          match type_definition {
            TypeDefinition::Object(object) => {
              self.enter_object_type(object);

              for field in &object.fields {
                self.enter_object_type_field(field, object);
                // TODO: More advanced setup for fields: arguments, lists, null/non-null, directives
                self.leave_object_type_field(field, object);
              }

              self.leave_object_type(object);
            }
            TypeDefinition::Scalar(scalar) => {
              self.enter_scalar_type(scalar);
              self.leave_scalar_type(scalar);
            }
            TypeDefinition::Enum(enum_) => {
              self.enter_enum_type(enum_);

              for value in &enum_.values {
                self.enter_enum_value(value, enum_);
                self.leave_enum_value(value, enum_);
              }

              self.leave_enum_type(enum_);
            }
            TypeDefinition::Union(union) => {
              self.enter_union_type(union);
              self.leave_union_type(union);
            }
            TypeDefinition::InputObject(input_object) => {
              self.enter_input_object_type(input_object);

              for field in &input_object.fields {
                self.enter_input_object_type_field(field, input_object);
                self.leave_input_object_type_field(field, input_object);
              }

              self.leave_input_object_type(input_object);
            }
            TypeDefinition::Interface(interface) => {
              self.enter_interface_type(interface);

              for field in &interface.fields {
                self.enter_interface_type_field(field, interface);
                self.leave_interface_type_field(field, interface);
              }

              self.leave_interface_type(interface);
            }
          }

          self.leave_type_definition(type_definition);
        }
        Definition::DirectiveDefinition(directive_definition) => {
          self.enter_directive_definition(directive_definition);
          self.leave_directive_definition(directive_definition);
        }
        Definition::TypeExtension(_type_extension) => {
          // TODO: implement this
          panic!("TypeExtension not supported at the moment");
        }
      }
    }

    self.leave_document(document);
  }

  fn enter_document(&mut self, _node: &'a Document<String>) {}
  fn leave_document(&mut self, _node: &'a Document<String>) {}

  fn enter_schema_definition(&mut self, _node: &'a SchemaDefinition<String>) {}
  fn leave_schema_definition(&mut self, _node: &'a SchemaDefinition<String>) {}

  fn enter_directive_definition(&mut self, _node: &'a DirectiveDefinition<String>) {}
  fn leave_directive_definition(&mut self, _node: &'a DirectiveDefinition<String>) {}

  fn enter_type_definition(&mut self, _node: &'a TypeDefinition<String>) {}
  fn leave_type_definition(&mut self, _node: &'a TypeDefinition<String>) {}

  fn enter_interface_type(&mut self, _node: &'a InterfaceType<String>) {}
  fn leave_interface_type(&mut self, _node: &'a InterfaceType<String>) {}

  fn enter_interface_type_field(&mut self, _node: &'a Field<String>, _type_: &'a InterfaceType<String>) {}
  fn leave_interface_type_field(&mut self, _node: &'a Field<String>, _type_: &'a InterfaceType<String>) {}

  fn enter_object_type(&mut self, _node: &'a ObjectType<String>) {}
  fn leave_object_type(&mut self, _node: &'a ObjectType<String>) {}

  fn enter_object_type_field(&mut self, _node: &'a Field<String>, _type_: &'a ObjectType<String>) {}
  fn leave_object_type_field(&mut self, _node: &'a Field<String>, _type_: &'a ObjectType<String>) {}

  fn enter_input_object_type(&mut self, _node: &'a InputObjectType<String>) {}
  fn leave_input_object_type(&mut self, _node: &'a InputObjectType<String>) {}

  fn enter_input_object_type_field(&mut self, _node: &'a InputValue<String>, _input_type: &'a InputObjectType<String>) {}
  fn leave_input_object_type_field(&mut self, _node: &'a InputValue<String>, _input_type: &'a InputObjectType<String>) {}

  fn enter_union_type(&mut self, _node: &'a UnionType<String>) {}
  fn leave_union_type(&mut self, _node: &'a UnionType<String>) {}

  fn enter_scalar_type(&mut self, _node: &'a ScalarType<String>) {}
  fn leave_scalar_type(&mut self, _node: &'a ScalarType<String>) {}

  fn enter_enum_type(&mut self, _node: &'a EnumType<String>) {}
  fn leave_enum_type(&mut self, _node: &'a EnumType<String>) {}

  fn enter_enum_value(&mut self, _node: &'a EnumValue<String>, _enum: &'a EnumType<String>) {}
  fn leave_enum_value(&mut self, _node: &'a EnumValue<String>, _enum: &'a EnumType<String>) {}
}

#[test]
fn visit_schema() {
  use graphql_parser::schema::{parse_schema};
  let schema_ast = parse_schema(r#"
    scalar Date

    type Query {
      user(id: ID!): User!
      users(filter: UsersFilter): [User!]!
      now: Date
    }

    input UsersFilter {
      name: String
    }

    type User implements Node {
      id: ID!
      name: String!
      role: Role!
    }

    interface Node {
      id: ID!
    }

    type Test {
      foo: String!
    }

    enum Role {
      USER
      ADMIN
    }

    union TestUnion = Test | User

    "#).expect("Failed to parse schema");

  struct TestVisitor {
    collected_object_type: Vec<String>,
    collected_scalar_type: Vec<String>,
    collected_union_type: Vec<String>,
    collected_input_type: Vec<String>,
    collected_enum_type: Vec<String>,
    collected_enum_value: Vec<String>,
    collected_interface_type: Vec<String>,
    collected_object_type_field: Vec<String>,
    collected_interface_type_field: Vec<String>,
    collected_input_type_fields: Vec<String>,
  }

  impl<'a> SchemaVisitor<'a> for TestVisitor {
    fn enter_object_type(&mut self, _node: &'a ObjectType<String>) {
      self.collected_object_type.push(_node.name.clone());
    }

    fn enter_object_type_field(&mut self, _node: &'a Field<String>, _type_: &'a ObjectType<String>) {
      let field_id = format!("{}.{}", _type_.name.as_str(), _node.name.as_str());
      self.collected_object_type_field.push(field_id);
    }

    fn enter_interface_type(&mut self, _node: &'a InterfaceType<String>) {
      self.collected_interface_type.push(_node.name.clone());
    }

    fn enter_interface_type_field(&mut self, _node: &'a Field<String>, _type_: &'a InterfaceType<String>) {
      self.collected_interface_type_field.push(_node.name.clone());
    }

    fn enter_scalar_type(&mut self, _node: &'a ScalarType<String>) {
      self.collected_scalar_type.push(_node.name.clone());
    }

    fn enter_union_type(&mut self, _node: &'a UnionType<String>) {
      self.collected_union_type.push(_node.name.clone());
    }

    fn enter_enum_type(&mut self, _node: &'a EnumType<String>) {
      self.collected_enum_type.push(_node.name.clone());
    }

    fn enter_enum_value(&mut self, _node: &'a EnumValue<String>, _enum: &'a EnumType<String>) {
      let enum_value_id = format!("{}.{}", _enum.name.as_str(), _node.name.as_str());
      self.collected_enum_value.push(enum_value_id);
    }

    fn enter_input_object_type(&mut self, _node: &'a InputObjectType<String>) {
      self.collected_input_type.push(_node.name.clone());
    }

    fn enter_input_object_type_field(&mut self, _node: &'a InputValue<String>, _input_type: &'a InputObjectType<String>) {
      let field_id = format!("{}.{}", _input_type.name.as_str(), _node.name.as_str());
      self.collected_input_type_fields.push(field_id);
    }
  }

  let mut visitor = TestVisitor {
    collected_object_type: Vec::new(),
    collected_interface_type: Vec::new(),
    collected_object_type_field: Vec::new(),
    collected_interface_type_field: Vec::new(),
    collected_scalar_type: Vec::new(),
    collected_union_type: Vec::new(),
    collected_enum_type: Vec::new(),
    collected_enum_value: Vec::new(),
    collected_input_type: Vec::new(),
    collected_input_type_fields: Vec::new(),
  };

  visitor.visit_schema_document(&schema_ast);

  assert_eq!(visitor.collected_object_type, vec!["Query", "User", "Test"]);
  assert_eq!(visitor.collected_object_type_field, vec!["Query.user", "Query.users", "Query.now", "User.id", "User.name", "User.role", "Test.foo"]);
  assert_eq!(visitor.collected_interface_type, vec!["Node"]);
  assert_eq!(visitor.collected_union_type, vec!["TestUnion"]);
  assert_eq!(visitor.collected_scalar_type, vec!["Date"]);
  assert_eq!(visitor.collected_enum_type, vec!["Role"]);
  assert_eq!(visitor.collected_enum_value, vec!["Role.USER", "Role.ADMIN"]);
  assert_eq!(visitor.collected_input_type, vec!["UsersFilter"]);
  assert_eq!(visitor.collected_input_type_fields, vec!["UsersFilter.name"]);
}
