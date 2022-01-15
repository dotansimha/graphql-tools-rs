use graphql_parser::Pos;

use super::ValidationRule;
use crate::ast::{visit_document, OperationVisitor, OperationVisitorContext};
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Overlapping fields can be merged
///
/// A selection set is only valid if all fields (including spreading any
/// fragments) either correspond to distinct response names or can be merged
/// without ambiguity.
///
/// See https://spec.graphql.org/draft/#sec-Field-Selection-Merging
pub struct OverlappingFieldsCanBeMerged;

#[derive(Debug)]
struct Conflict(ConflictReason, Vec<Pos>, Vec<Pos>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConflictReason(String, ConflictReasonMessage);

#[derive(Debug)]
struct AstAndDef<'a>(Option<&'a str>, &'a Field, Option<&'a Type>);

type AstAndDefCollection<'a> = OrderedMap<&'a str, Vec<AstAndDef<'a>>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConflictReasonMessage {
    Message(String),
    Nested(Vec<ConflictReason>),
}

struct PairSet {
    data: HashMap<String, HashMap<String, bool>>,
}

struct OrderedMap<K, V> {
    data: HashMap<K, V>,
    insert_order: Vec<K>,
}

struct OrderedMapIter<'a, K: 'a, V: 'a> {
    map: &'a HashMap<K, V>,
    inner: ::std::slice::Iter<'a, K>,
}

impl<K: Eq + Hash + Clone, V> OrderedMap<K, V> {
    fn new() -> OrderedMap<K, V> {
        OrderedMap {
            data: HashMap::new(),
            insert_order: Vec::new(),
        }
    }

    fn iter(&self) -> OrderedMapIter<K, V> {
        OrderedMapIter {
            map: &self.data,
            inner: self.insert_order.iter(),
        }
    }

    fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.get(k)
    }

    fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.get_mut(k)
    }

    fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.contains_key(k)
    }

    fn insert(&mut self, k: K, v: V) -> Option<V> {
        let result = self.data.insert(k.clone(), v);
        if result.is_none() {
            self.insert_order.push(k);
        }
        result
    }
}

impl<'a, K: Eq + Hash + 'a, V: 'a> Iterator for OrderedMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .and_then(|key| self.map.get(key).map(|value| (key, value)))
    }
}

impl PairSet {
    fn new() -> PairSet {
        PairSet {
            data: HashMap::new(),
        }
    }

    pub fn contains(&self, a: &String, b: &String, mutex: bool) -> bool {
        if let Some(result) = self.data.get(a).and_then(|s| s.get(b)) {
            if !mutex {
                !result
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn insert(&mut self, a: &String, b: &String, mutex: bool) {
        self.data
            .entry(a.clone())
            .or_insert_with(HashMap::new)
            .insert(b.clone(), mutex);

        self.data
            .entry(b.clone())
            .or_insert_with(HashMap::new)
            .insert(a.clone(), mutex);
    }
}

struct OverlappingFieldsCanBeMergedHelper {
    error_context: ValidationErrorContext,
    named_fragments: HashMap<String, FragmentDefinition>,
    compared_fragments: PairSet,
}

impl OverlappingFieldsCanBeMergedHelper {
    fn new() -> Self {
        Self {
            error_context: ValidationErrorContext::new(),
            named_fragments: HashMap::new(),
            compared_fragments: PairSet::new(),
        }
    }
}

impl<'a> OperationVisitor<'a, OverlappingFieldsCanBeMergedHelper> for OverlappingFieldsCanBeMerged {}

impl ValidationRule for OverlappingFieldsCanBeMerged {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = OverlappingFieldsCanBeMergedHelper::new();

        visit_document(
            &mut OverlappingFieldsCanBeMerged {},
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut helper, &ctx.operation, &ctx.schema),
        );

        helper.error_context.errors
    }
}

#[test]
fn unique_fields() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment uniqueFields on Dog {
          name
          nickname
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn identical_fields() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment mergeIdenticalFields on Dog {
          name
          name
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn identical_fields_and_identical_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment mergeIdenticalFieldsWithIdenticalArgs on Dog {
          doesKnowCommand(dogCommand: SIT)
          doesKnowCommand(dogCommand: SIT)
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn identical_fields_and_identical_directives() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment mergeSameFieldsWithSameDirectives on Dog {
          name @include(if: true)
          name @include(if: true)
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn different_args_different_aliases() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment differentArgsWithDifferentAliases on Dog {
          knowsSit: doesKnowCommand(dogCommand: SIT)
          knowsDown: doesKnowCommand(dogCommand: DOWN)
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn different_directives_different_aliases() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment differentDirectivesWithDifferentAliases on Dog {
          nameIfTrue: name @include(if: true)
          nameIfFalse: name @include(if: false)
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn different_skip_include_directives() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment differentDirectivesWithDifferentAliases on Dog {
          name @include(if: true)
          name @include(if: false)
        }",
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn same_alias_different_field_target() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment sameAliasesWithDifferentFieldTargets on Dog {
          fido: name
          fido: nickname
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"fido\" conflict because \"name\" and \"nickname\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn same_alias_non_overlapping_field_target() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment sameAliasesWithDifferentFieldTargets on Pet {
          ... on Dog {
            name
          }
          ... on Cat {
            name: nickname
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn alias_masking_direct_access() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment aliasMaskingDirectFieldAccess on Dog {
          name: nickname
          name
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"name\" conflict because \"nickname\" and \"name\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn different_args_second_adds() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment conflictingArgs on Dog {
          doesKnowCommand
          doesKnowCommand(dogCommand: HEEL)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"doesKnowCommand\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn different_args_declared_on_first() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment conflictingArgs on Dog {
          doesKnowCommand(dogCommand: SIT)
          doesKnowCommand
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"doesKnowCommand\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn different_arg_values() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment conflictingArgs on Dog {
          doesKnowCommand(dogCommand: SIT)
          doesKnowCommand(dogCommand: HEEL)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"doesKnowCommand\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn conflicting_arg_names() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment conflictingArgs on Dog {
          isAtLocation(x: 0)
          isAtLocation(y: 0)
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"isAtLocation\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn allow_different_args_when_possible_with_different_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment conflictingArgs on Pet {
          ... on Dog {
            name(surname: true)
          }
          ... on Cat {
            name
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn conflict_in_fragment_spread() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "query {
          ...A
          ...B
        }
        fragment A on Type {
          x: a
        }
        fragment B on Type {
          x: b
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"x\" conflict because \"a\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

// TODO: Fix. At the moment we are not able to run this one due to missing flattening.
#[test]
#[ignore]
fn deep_conflict() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          field {
            x: a
          }
          field {
            x: b
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"x\" conflict because \"a\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn report_each_conflict_once() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          f1 {
            ...A
            ...B
          }
          f2 {
            ...B
            ...A
          }
          f3 {
            ...A
            ...B
            x: c
          }
        }
        fragment A on Type {
          x: a
        }
        fragment B on Type {
          x: b
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 3);
    assert_eq!(messages, vec![
      "Fields \"x\" conflict because \"a\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional.",
      "Fields \"x\" conflict because \"b\" and \"a\" are different fields. Use different aliases on the fields to fetch both if this was intentional.",
      "Fields \"x\" conflict because \"a\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[cfg(test)]
pub static OVERLAPPING_RULE_TEST_SCHEMA: &str = "
interface SomeBox {
  deepBox: SomeBox
  unrelatedField: String
}
type StringBox implements SomeBox {
  scalar: String
  deepBox: StringBox
  unrelatedField: String
  listStringBox: [StringBox]
  stringBox: StringBox
  intBox: IntBox
}
type IntBox implements SomeBox {
  scalar: Int
  deepBox: IntBox
  unrelatedField: String
  listStringBox: [StringBox]
  stringBox: StringBox
  intBox: IntBox
}
interface NonNullStringBox1 {
  scalar: String!
}
type NonNullStringBox1Impl implements SomeBox & NonNullStringBox1 {
  scalar: String!
  unrelatedField: String
  deepBox: SomeBox
}
interface NonNullStringBox2 {
  scalar: String!
}
type NonNullStringBox2Impl implements SomeBox & NonNullStringBox2 {
  scalar: String!
  unrelatedField: String
  deepBox: SomeBox
}
type Connection {
  edges: [Edge]
}
type Edge {
  node: Node
}
type Node {
  id: ID
  name: String
}
type Query {
  someBox: SomeBox
  connection: Connection
}";

#[test]
fn conflicting_return_types_which_potentially_overlap() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ...on IntBox {
              scalar
            }
            ...on NonNullStringBox1 {
              scalar
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"scalar\" conflict because they return conflicting types \"Int\" and \"String!\". Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn compatible_return_shapes_on_different_return_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on SomeBox {
              deepBox {
                unrelatedField
              }
            }
            ... on StringBox {
              deepBox {
                unrelatedField
              }
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn disallows_differing_return_types_despite_no_overlap() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on IntBox {
              scalar
            }
            ... on StringBox {
              scalar
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"scalar\" conflict because they return conflicting types \"Int\" and \"String\". Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn reports_correctly_when_a_non_exclusive_follows_an_exclusive() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on IntBox {
              deepBox {
                ...X
              }
            }
          }
          someBox {
            ... on StringBox {
              deepBox {
                ...Y
              }
            }
          }
          memoed: someBox {
            ... on IntBox {
              deepBox {
                ...X
              }
            }
          }
          memoed: someBox {
            ... on StringBox {
              deepBox {
                ...Y
              }
            }
          }
          other: someBox {
            ...X
          }
          other: someBox {
            ...Y
          }
        }
        fragment X on SomeBox {
          scalar
        }
        fragment Y on SomeBox {
          scalar: unrelatedField
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"other\" conflict because subfields \"scalar\" conflict because \"scalar\" and \"unrelatedField\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn disallows_differing_return_type_nullability_despite_no_overlap() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on NonNullStringBox1 {
              scalar
            }
            ... on StringBox {
              scalar
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"scalar\" conflict because they return conflicting types \"String!\" and \"String\". Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn disallows_differing_return_type_list_despite_no_overlap() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on IntBox {
              box: listStringBox {
                scalar
              }
            }
            ... on StringBox {
              box: stringBox {
                scalar
              }
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"box\" conflict because they return conflicting types \"[StringBox]\" and \"StringBox\". Use different aliases on the fields to fetch both if this was intentional."
    ]);

    let errors = test_operation_with_schema(
        "{
            someBox {
              ... on IntBox {
                box: stringBox {
                  scalar
                }
              }
              ... on StringBox {
                box: listStringBox {
                  scalar
                }
              }
            }
          }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"box\" conflict because they return conflicting types \"StringBox\" and \"[StringBox]\". Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn disallows_differing_subfields() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on IntBox {
              box: stringBox {
                val: scalar
                val: unrelatedField
              }
            }
            ... on StringBox {
              box: stringBox {
                val: scalar
              }
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"val\" conflict because \"scalar\" and \"unrelatedField\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn disallows_differing_deep_return_types_despite_no_overlap() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on IntBox {
              box: stringBox {
                scalar
              }
            }
            ... on StringBox {
              box: intBox {
                scalar
              }
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"box\" conflict because subfields \"scalar\" conflict because they return conflicting types \"String\" and \"Int\". Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn allows_non_conflicting_overlapping_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ... on IntBox {
              scalar: unrelatedField
            }
            ... on StringBox {
              scalar
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn same_wrapped_scalar_return_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ...on NonNullStringBox1 {
              scalar
            }
            ...on NonNullStringBox2 {
              scalar
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn allows_inline_fragments_without_type_condition() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          a
          ... {
            a
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn compares_deep_types_including_list() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          connection {
            ...edgeID
            edges {
              node {
                id: name
              }
            }
          }
        }
        fragment edgeID on Connection {
          edges {
            node {
              id
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"edges\" conflict because subfields \"node\" conflict because subfields \"id\" conflict because \"name\" and \"id\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
    ]);
}

#[test]
fn ignores_unknown_types() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_with_schema(
        "{
          someBox {
            ...on UnknownType {
              scalar
            }
            ...on NonNullStringBox2 {
              scalar
            }
          }
        }",
        OVERLAPPING_RULE_TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn does_not_infinite_loop_on_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "fragment fragA on Human { name, relatives { name, ...fragA } }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn does_not_infinite_loop_on_immediately_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors =
        test_operation_without_schema("fragment fragA on Human { name, ...fragA }", &mut plan);

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn does_not_infinite_loop_on_transitively_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "
        fragment fragA on Human { name, ...fragB }
        fragment fragB on Human { name, ...fragC }
        fragment fragC on Human { name, ...fragA }
      ",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn finds_invalid_case_even_with_immediately_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "
        fragment sameAliasesWithDifferentFieldTargets on Dog {
          ...sameAliasesWithDifferentFieldTargets
          fido: name
          fido: nickname
        }
      ",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"fido\" conflict because \"name\" and \"nickname\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
    ]);
}
