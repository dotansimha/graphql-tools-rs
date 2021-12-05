use graphql_parser::schema::{Document, Definition, SchemaDefinition, DirectiveDefinition, TypeDefinition, ObjectType, ScalarType, EnumType, Field, EnumValue, UnionType, InputObjectType, InterfaceType};

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
  let schema_ast = parse_schema("type Query { foo: String! }").expect("Failed to parse schema");

  struct TestVisitor;

  impl<'a> SchemaVisitor<'a> for TestVisitor {}

  let mut visitor = TestVisitor {};
  visitor.visit_schema_document(&schema_ast);
}
