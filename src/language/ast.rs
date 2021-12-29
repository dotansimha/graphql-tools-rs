mod kind;
mod source;
mod token_kind;
use crate::kind::Kind;
use crate::source::Source;
use crate::token_kind::TokenKind;
use std::collections::HashMap;

/// Represents a range of characters represented by a lexical token
/// within a Source.
pub struct Token {
	/// The kind of Token.
	kind: TokenKind,
	/// The character offset at which this Node begins.
	start: usize,
	/// The character offset at which this Node ends.
	end: usize,
	/// The 1-indexed line number on which this Token appears.
	line: usize,
	/// The 1-indexed column number at which this Token begins.
	column: usize,
	/// For non-punctuation tokens, represents the interpreted value of the token.
	///
	/// Note: is undefined for punctuation tokens, but typed as string for
	/// convenience in the parser.
	value: String,
	/// Tokens exist as nodes in a double-linked-list amongst all tokens
	/// including ignored tokens. <SOF> is always the first node and <EOF>
	/// the last.
	prev: Option<Token>,
	next: Option<Token>,
}

impl Token {
	pub fn new(
		kind: TokenKind,
		start: usize,
		end: usize,
		line: usize,
		column: usize,
		value: String,
	) -> Token {
		Token {
			kind,
			start,
			end,
			line,
			column,
			value,
			prev: None,
			next: None,
		}
	}

	pub fn toJSON(&self) -> HashMap<String, String> {
		let mut map = HashMap > ::new();
		map.insert("kind", self.kind);
		map.insert("value", self.value);
		map.insert("line", self.line.to_string());
		map.insert("column", self.column.to_string());
		return map;
	}
}

/// Contains a range of UTF-8 character offsets and token references that
/// identify the region of the source from which the AST derived.
pub struct Location {
	/// The character offset at which this Node begins.
	pub start: usize,
	/// The character offset at which this Node ends.
	pub end: usize,
	/// The Token at which this Node begins.
	pub start_token: Token,
	/// The Token at which this Node ends.
	pub end_token: Token,
	/// The Source document the AST represents.
	pub source: Source,
}

impl Location {
	/// Creates a new Location object.
	pub fn new(start_token: Token, end_token: Token, source: Source) -> Location {
		Location {
			start: start_token.start,
			end: end_token.end,
			start_token,
			end_token,
			source,
		}
	}

	pub fn toJSON(&self) -> Map<String, String> {
		let mut map = HashMap::new();
		map.insert("start", self.start.to_string());
		map.insert("end", self.end.to_string());
		return map;
	}
}

/// The list of all possible AST node types.
pub enum ASTNode {
	NameNode,
	DocumentNode,
	OperationDefinitionNode,
	VariableDefinitionNode,
	VariableNode,
	SelectionSetNode,
	FieldNode,
	ArgumentNode,
	FragmentSpreadNode,
	InlineFragmentNode,
	FragmentDefinitionNode,
	IntValueNode,
	FloatValueNode,
	StringValueNode,
	BooleanValueNode,
	NullValueNode,
	EnumValueNode,
	ListValueNode,
	ObjectValueNode,
	ObjectFieldNode,
	DirectiveNode,
	NamedTypeNode,
	ListTypeNode,
	NonNullTypeNode,
	SchemaDefinitionNode,
	OperationTypeDefinitionNode,
	ScalarTypeDefinitionNode,
	ObjectTypeDefinitionNode,
	FieldDefinitionNode,
	InputValueDefinitionNode,
	InterfaceTypeDefinitionNode,
	UnionTypeDefinitionNode,
	EnumTypeDefinitionNode,
	EnumValueDefinitionNode,
	InputObjectTypeDefinitionNode,
	DirectiveDefinitionNode,
	SchemaExtensionNode,
	ScalarTypeExtensionNode,
	ObjectTypeExtensionNode,
	InterfaceTypeExtensionNode,
	UnionTypeExtensionNode,
	EnumTypeExtensionNode,
	InputObjectTypeExtensionNode,
}

pub const QueryDocumentKeys: HashMap<String, Vec<String>> = [
	(Name, []),
	(Document, ["definitions"]),
	(
		OperationDefinition,
		["name", "variableDefinitions", "directives", "selectionSet"],
	),
	(
		VariableDefinition,
		["variable", "type", "defaultValue", "directives"],
	),
	(Variable, ["name"]),
	(SelectionSet, ["selections"]),
	(
		Field,
		["alias", "name", "arguments", "directives", "selectionSet"],
	),
	(Argument, ["name", "value"]),
	(FragmentSpread, ["name", "directives"]),
	(
		InlineFragment,
		["typeCondition", "directives", "selectionSet"],
	),
	(
		FragmentDefinition,
		[
			"name",
			// Note: fragment variable definitions are deprecated and will removed in v17.0.0
			"variableDefinitions",
			"typeCondition",
			"directives",
			"selectionSet",
		],
	),
	(IntValue, []),
	(FloatValue, []),
	(StringValue, []),
	(BooleanValue, []),
	(NullValue, []),
	(EnumValue, []),
	(ListValue, ["values"]),
	(ObjectValue, ["fields"]),
	(ObjectField, ["name", "value"]),
	(Directive, ["name", "arguments"]),
	(NamedType, ["name"]),
	(ListType, ["type"]),
	(NonNullType, ["type"]),
	(
		SchemaDefinition,
		["description", "directives", "operationTypes"],
	),
	(OperationTypeDefinition, ["type"]),
	(ScalarTypeDefinition, ["description", "name", "directives"]),
	(
		ObjectTypeDefinition,
		["description", "name", "interfaces", "directives", "fields"],
	),
	(
		FieldDefinition,
		["description", "name", "arguments", "type", "directives"],
	),
	(
		InputValueDefinition,
		["description", "name", "type", "defaultValue", "directives"],
	),
	(
		InterfaceTypeDefinition,
		["description", "name", "interfaces", "directives", "fields"],
	),
	(
		UnionTypeDefinition,
		["description", "name", "directives", "types"],
	),
	(
		EnumTypeDefinition,
		["description", "name", "directives", "values"],
	),
	(EnumValueDefinition, ["description", "name", "directives"]),
	(
		InputObjectTypeDefinition,
		["description", "name", "directives", "fields"],
	),
	(
		DirectiveDefinition,
		["description", "name", "arguments", "locations"],
	),
	(SchemaExtension, ["directives", "operationTypes"]),
	(ScalarTypeExtension, ["name", "directives"]),
	(
		ObjectTypeExtension,
		["name", "interfaces", "directives", "fields"],
	),
	(
		InterfaceTypeExtension,
		["name", "interfaces", "directives", "fields"],
	),
	(UnionTypeExtension, ["name", "directives", "types"]),
	(EnumTypeExtension, ["name", "directives", "values"]),
	(InputObjectTypeExtension, ["name", "directives", "fields"]),
];

/// Name
pub struct NameNode {
	kind: Kind::NAME,
	loc: Option<Location>,
	value: String,
}

/// Document
pub struct DocumentNode {
	kind: Kind::DOCUMENT,
	loc: Option<Location>,
	definitions: Vec<DefinitionNode>, // not sure how to make this read-only like TS
}

pub enum DefinitionNode {
	ExecutableDefinitionNode,
	TypeSystemDefinitionNode,
	TypeSystemExtensionNode,
}

pub enum ExecutableDefinitionNode {
	OperationDefinitionNode,
	FragmentDefinitionNode,
}

pub enum OperationTypeNode {
	QUERY = "query",
	MUTATION = "mutation",
	SUBSCRIPTION = "subscription",
}
pub struct OperationDefinitionNode {
	kind: Kind::OPERATION_DEFINITION,
	loc: Option<Location>,
	operation: OperationTypeNode,
	name: Option<NameNode>,
	variableDefinitions: Option<Vec<VariableDefinitionNode>>,
	directives: Option<Vec<DirectiveNode>>,
	selectionSet: SelectionSetNode,
}

pub struct VariableNode {
	kind: Kind::VARIABLE,
	loc: Option<Location>,
	name: NameNode,
}

pub struct ConstArgumentNode {
	kind: Kind::ARGUMENT,
	loc: Option<Location>,
	name: NameNode,
	value: ConstValueNode,
}
pub struct VariableDefinitionNode {
	kind: Kind::VARIABLE_DEFINITION,
	loc: Option<Location>,
	variable: VariableNode,
	node_type: TypeNode,
	defaultValue: Option<ConstValueNode>,
	directives: Option<Vec<ConstDirectiveNode>>,
}

pub enum SelectionNode {
	FieldNode,
	FragmentSpreadNode,
	InlineFragmentNode,
}

pub struct SelectionSetNode {
	kind: Kind::SELECTION_SET,
	loc: Option<Location>,
	selections: Option<Vec<SelectionNode>>,
}

pub struct ArgumentNode {
	kind: Kind::ARGUMENT,
	loc: Option<Location>,
	name: NameNode,
	value: ValueNode,
}

pub struct FieldNode {
	kind: Kind::FIELD,
	loc: Option<Location>,
	alias: Option<NameNode>,
	name: NameNode,
	arguments: Option<Vec<ArgumentNode>>,
	directives: Option<Vec<DirectiveNode>>,
	selectionSet: Option<SelectionSetNode>,
}

/// Fragments
pub struct FragmentSpreadNode {
	kind: Kind::FRAGMENT_SPREAD,
	loc: Option<Location>,
	name: NameNode,
	directives: Option<Vec<DirectiveNode>>,
}

pub struct InlineFragmentNode {
	kind: Kind::INLINE_FRAGMENT,
	loc: Option<Location>,
	typeCondition: Option<NamedTypeNode>,
	directives: Option<Vec<DirectiveNode>>,
	selectionSet: SelectionSetNode,
}

pub struct FragmentDefinitionNode {
	kind: Kind::FRAGMENT_DEFINITION,
	loc: Option<Location>,
	name: NameNode,
	/// @deprecated variableDefinitions will be removed in v17.0.0
	variableDefinitions: Option<Vec<VariableDefinitionNode>>,
	typeCondition: NamedTypeNode,
	directives: Option<Vec<DirectiveNode>>,
	selectionSet: SelectionSetNode,
}

/// Values
pub enum ValueNode {
	VariableNode,
	IntValueNode,
	FloatValueNode,
	StringValueNode,
	BooleanValueNode,
	NullValueNode,
	EnumValueNode,
	ListValueNode,
	ObjectValueNode,
}
pub enum ConstValueNode {
	IntValueNode,
	FloatValueNode,
	StringValueNode,
	BooleanValueNode,
	NullValueNode,
	EnumValueNode,
	ConstListValueNode,
	ConstObjectValueNode,
}

pub struct IntValueNode {
	kind: Kind::INT,
	loc: Option<Location>,
	value: String,
}
pub struct FloatValueNode {
	kind: Kind::FLOAT,
	loc: Option<Location>,
	value: String,
}
pub struct StringValueNode {
	kind: Kind::STRING,
	loc: Option<Location>,
	value: String,
	block: Option<Bool>,
}
pub struct BooleanValueNode {
	kind: Kind::BOOLEAN,
	loc: Option<Location>,
	value: Option<Bool>,
}
pub struct NullValueNode {
	kind: Kind::NULL,
	loc: Option<Location>,
}
pub struct EnumValueNode {
	kind: Kind::ENUM,
	loc: Option<Location>,
	value: String,
}
pub struct ListValueNode {
	kind: Kind::LIST,
	loc: Option<Location>,
	values: Option<Vec<ValueNode>>,
}
pub struct ConstListValueNode {
	kind: Kind::LIST,
	loc: Option<Location>,
	values: Option<Vec<ConstValueNode>>,
}
pub struct ObjectValueNode {
	kind: Kind::OBJECT,
	loc: Option<Location>,
	fields: Option<Vec<ObjectFieldNode>>,
}
pub struct ConstObjectValueNode {
	kind: Kind::OBJECT,
	loc: Option<Location>,
	fields: Option<Vec<ConstObjectFieldNode>>,
}
pub struct ObjectFieldNode {
	kind: Kind::OBJECT_FIELD,
	loc: Option<Location>,
	name: NameNode,
	value: ValueNode,
}
pub struct ConstObjectFieldNode {
	kind: Kind::OBJECT_FIELD,
	loc: Option<Location>,
	name: NameNode,
	value: ConstValueNode,
}

/// Directives
pub struct DirectiveNode {
	kind: Kind::DIRECTIVE,
	loc: Option<Location>,
	name: NameNode,
	arguments: Option<Vec<ArgumentNode>>,
}
pub struct ConstDirectiveNode {
	kind: Kind::DIRECTIVE,
	loc: Option<Location>,
	name: NameNode,
	arguments: Option<Vec<ConstArgumentNode>>,
}

/// Type Reference
pub enum TypeNode {
	NamedTypeNode,
	ListTypeNode,
	NonNullTypeNode,
}
pub struct NamedTypeNode {
	kind: Kind::NAMED_TYPE,
	loc: Option<Location>,
	name: NameNode,
}
pub struct ListTypeNode {
	kind: Kind::LIST_TYPE,
	loc: Option<Location>,
	node_type: TypeNode,
}

enum NonNullTypeNode_NodeType {
	NamedTypeNode,
	ListTypeNode,
}

pub struct NonNullTypeNode {
	kind: Kind::NON_NULL_TYPE,
	loc: Option<Location>,
	node_type: NonNullTypeNode_NodeType,
}

/// Type System Definitions
pub enum TypeSystemDefinitionNode {
	SchemaDefinitionNode,
	TypeDefinitionNode,
	DirectiveDefinitionNode,
}
pub struct SchemaDefinitionNode {
	kind: Kind::SCHEMA_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	directives: Option<Vec<ConstDirectiveNode>>,
	operationTypes: Option<Vec<OperationTypeDefinitionNode>>,
}
pub struct OperationTypeDefinitionNode {
	kind: Kind::OPERATION_TYPE_DEFINITION,
	loc: Option<Location>,
	operation: OperationTypeNode,
	node_type: NamedTypeNode,
}

/// Type Definition
pub enum TypeDefinitionNode {
	ScalarTypeDefinitionNode,
	ObjectTypeDefinitionNode,
	InterfaceTypeDefinitionNode,
	UnionTypeDefinitionNode,
	EnumTypeDefinitionNode,
	InputObjectTypeDefinitionNode,
}
pub struct ScalarTypeDefinitionNode {
	kind: Kind::SCALAR_TYPE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
}
pub struct ObjectTypeDefinitionNode {
	kind: Kind::OBJECT_TYPE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	interfaces: Option<Vec<NamedTypeNode>>,
	directives: Option<Vec<ConstDirectiveNode>>,
	fields: Option<Vec<FieldDefinitionNode>>,
}
pub struct FieldDefinitionNode {
	kind: Kind::FIELD_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	arguments: Option<Vec<InputValueDefinitionNode>>,
	node_type: TypeNode,
	directives: Option<Vec<ConstDirectiveNode>>,
}
pub struct InputValueDefinitionNode {
	kind: Kind::INPUT_VALUE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	node_type: TypeNode,
	defaultValue: Option<ConstValueNode>,
	directives: Option<Vec<ConstDirectiveNode>>,
}
pub struct InterfaceTypeDefinitionNode {
	kind: Kind::INTERFACE_TYPE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	interfaces: Option<Vev<NamedTypeNode>>,
	directives: Option<Vec<ConstDirectiveNode>>,
	fields: Option<Vec<FieldDefinitionNode>>,
}
pub struct UnionTypeDefinitionNode {
	kind: Kind::UNION_TYPE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
	types: Option<Vec<NamedTypeNode>>,
}
pub struct EnumTypeDefinitionNode {
	kind: Kind::ENUM_TYPE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
	values: Option<Vec<EnumValueDefinitionNode>>,
}
pub struct EnumValueDefinitionNode {
	kind: Kind::ENUM_VALUE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
}
pub struct InputObjectTypeDefinitionNode {
	kind: Kind::INPUT_OBJECT_TYPE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
	fields: Option<Vec<InputValueDefinitionNode>>,
}
/// Directive Definitions
pub struct DirectiveDefinitionNode {
	kind: Kind::DIRECTIVE_DEFINITION,
	loc: Option<Location>,
	description: Option<StringValueNode>,
	name: NameNode,
	arguments: Option<Vec<InputValueDefinitionNode>>,
	repeatable: Bool,
	locations: Option<Vec<NameNode>>,
}

/// Type System Extensions
pub enum TypeSystemExtensionNode {
	SchemaExtensionNode,
	TypeExtensionNode,
}
pub struct SchemaExtensionNode {
	kind: Kind::SCHEMA_EXTENSION,
	loc: Option<Location>,
	directives: Option<Vec<ConstDirectiveNode>>,
	operationTypes: Option<Vec<OperationTypeDefinitionNode>>,
}

/// Type Extensions
pub enum TypeExtensionNode {
	ScalarTypeExtensionNode,
	ObjectTypeExtensionNode,
	InterfaceTypeExtensionNode,
	UnionTypeExtensionNode,
	EnumTypeExtensionNode,
	InputObjectTypeExtensionNode,
}
pub struct ScalarTypeExtensionNode {
	kind: Kind::SCALAR_TYPE_EXTENSION,
	loc: Option<Location>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
}
pub struct ObjectTypeExtensionNode {
	kind: Kind::OBJECT_TYPE_EXTENSION,
	loc: Option<Location>,
	name: NameNode,
	interfaces: Option<Vec<NamedTypeNode>>,
	directives: Option<Vec<ConstDirectiveNode>>,
	fields: Option<Vec<FieldDefinitionNode>>,
}
pub struct InterfaceTypeExtensionNode {
	kind: Kind::INTERFACE_TYPE_EXTENSION,
	loc: Option<Location>,
	name: NameNode,
	interfaces: Option<Vec<NamedTypeNode>>,
	directives: Option<Vec<ConstDirectiveNode>>,
	fields: Option<Vec<FieldDefinitionNode>>,
}
pub struct UnionTypeExtensionNode {
	kind: Kind::UNION_TYPE_EXTENSION,
	loc: Option<Location>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
	types: Option<Vec<NamedTypeNode>>,
}
pub struct EnumTypeExtensionNode {
	kind: Kind::ENUM_TYPE_EXTENSION,
	loc: Option<Location>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
	values: Option<Vec<EnumValueDefinitionNode>>,
}
pub struct InputObjectTypeExtensionNode {
	kind: Kind::INPUT_OBJECT_TYPE_EXTENSION,
	loc: Option<Location>,
	name: NameNode,
	directives: Option<Vec<ConstDirectiveNode>>,
	fields: Option<Vec<InputValueDefinitionNode>>,
}
