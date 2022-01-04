use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};
use std::collections::HashMap;

/// Overlapping fields can be merged
///
/// A selection set is only valid if all fields (including spreading any
/// fragments) either correspond to distinct response names or can be merged
/// without ambiguity.
///
/// See https://spec.graphql.org/draft/#sec-Field-Selection-Merging
pub struct OverlappingFieldsCanBeMerged;

/**
 * Algorithm:
 *
 * Conflicts occur when two fields exist in a query which will produce the same
 * response name, but represent differing values, thus creating a conflict.
 * The algorithm below finds all conflicts via making a series of comparisons
 * between fields. In order to compare as few fields as possible, this makes
 * a series of comparisons "within" sets of fields and "between" sets of fields.
 *
 * Given any selection set, a collection produces both a set of fields by
 * also including all inline fragments, as well as a list of fragments
 * referenced by fragment spreads.
 *
 * A) Each selection set represented in the document first compares "within" its
 * collected set of fields, finding any conflicts between every pair of
 * overlapping fields.
 * Note: This is the *only time* that a the fields "within" a set are compared
 * to each other. After this only fields "between" sets are compared.
 *
 * B) Also, if any fragment is referenced in a selection set, then a
 * comparison is made "between" the original set of fields and the
 * referenced fragment.
 *
 * C) Also, if multiple fragments are referenced, then comparisons
 * are made "between" each referenced fragment.
 *
 * D) When comparing "between" a set of fields and a referenced fragment, first
 * a comparison is made between each field in the original set of fields and
 * each field in the the referenced set of fields.
 *
 * E) Also, if any fragment is referenced in the referenced selection set,
 * then a comparison is made "between" the original set of fields and the
 * referenced fragment (recursively referring to step D).
 *
 * F) When comparing "between" two fragments, first a comparison is made between
 * each field in the first referenced set of fields and each field in the the
 * second referenced set of fields.
 *
 * G) Also, any fragments referenced by the first must be compared to the
 * second, and any fragments referenced by the second must be compared to the
 * first (recursively referring to step F).
 *
 * H) When comparing two fields, if both have selection sets, then a comparison
 * is made "between" both selection sets, first comparing the set of fields in
 * the first selection set with the set of fields in the second.
 *
 * I) Also, if any fragment is referenced in either selection set, then a
 * comparison is made "between" the other set of fields and the
 * referenced fragment.
 *
 * J) Also, if two fragments are referenced in both selection sets, then a
 * comparison is made "between" the two fragments.
 *
 */

struct PairSet {
    pairs: HashMap<String, HashMap<String, bool>>,
}

impl PairSet {
    fn new() -> PairSet {
        PairSet {
            pairs: HashMap::new(),
        }
    }

    pub fn add(&mut self, a: &str, b: &str, are_mutually_exclusive: bool) -> () {
        let (key1, key2) = if a < b { (a, b) } else { (b, a) };
        let map = self.pairs.entry(key1.to_string()).or_insert(HashMap::new());
        map.insert(key2.to_string(), are_mutually_exclusive);
    }

    pub fn has(&self, a: &str, b: &str, are_mutually_exclusive: bool) -> bool {
        let (key1, key2) = if a < b { (a, b) } else { (b, a) };

        if let Some(map) = self.pairs.get(key1) {
            if let Some(child) = map.get(key2) {
                if are_mutually_exclusive {
                    return true;
                } else {
                    return are_mutually_exclusive == *child;
                }
            }
        }

        false
    }
}

struct FindOverlappingFieldsThatCanBeMergedHelper<'a> {
    compared_fragment_pairs: PairSet,
    validation_context: &'a ValidationErrorContext<'a>,
}

struct ConflictReasonMessage();

impl<'a> FindOverlappingFieldsThatCanBeMergedHelper<'a> {
    fn new(ctx: &'a ValidationErrorContext<'a>) -> Self {
        Self {
            compared_fragment_pairs: PairSet::new(),
            validation_context: ctx,
        }
    }

    // fn find_onflicts_within_selection_set()
}

impl<'a> QueryVisitor<FindOverlappingFieldsThatCanBeMergedHelper<'a>>
    for OverlappingFieldsCanBeMerged
{
    fn enter_selection_set(
        &self,
        node: &SelectionSet,
        ctx: &mut FindOverlappingFieldsThatCanBeMergedHelper<'a>,
    ) {
    }
}

impl ValidationRule for OverlappingFieldsCanBeMerged {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut error_context = ValidationErrorContext::new(ctx);
        let mut helper = FindOverlappingFieldsThatCanBeMergedHelper::new(&error_context);
        self.visit_document(&ctx.operation.clone(), &mut helper);

        error_context.errors
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

#[test]
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
fn deep_conflict_with_multiple_issues() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          field {
            x: a
            y: c
          },
          field {
            x: b
            y: d
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"field\" conflict because subfields \"x\" conflict because \"a\" and \"b\" are different fields and subfields \"y\" conflict because \"c\" and \"d\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn very_deep_conflict() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          field {
            deepField {
              x: a
            }
          },
          field {
            deepField {
              x: b
            }
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"deepField\" conflict because subfields \"x\" conflict because \"a\" and \"b\" are different fields and subfields \"y\" conflict because \"c\" and \"d\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn reports_deep_conflict_to_nearest_common_ancestor() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          field {
            deepField {
              x: a
            }
            deepField {
              x: b
            }
          },
          field {
            deepField {
              y
            }
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"deepField\" conflict because subfields \"x\" conflict because \"a\" and \"b\" are different fields and subfields \"y\" conflict because \"c\" and \"d\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn reports_deep_conflict_to_nearest_common_ancestor_in_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          field {
            ...F
          }
          field {
            ...F
          }
        }
        fragment F on T {
          deepField {
            deeperField {
              x: a
            }
            deeperField {
              x: b
            }
          },
          deepField {
            deeperField {
              y
            }
          }
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"deeperField\" conflict because subfields \"x\" conflict because \"a\" and \"b\" are different fields and subfields \"y\" conflict because \"c\" and \"d\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn reports_deep_conflict_in_nested_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          field {
            ...F
          }
          field {
            ...I
          }
        }
        fragment F on T {
          x: a
          ...G
        }
        fragment G on T {
          y: c
        }
        fragment I on T {
          y: d
          ...J
        }
        fragment J on T {
          x: b
        }",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"field\" conflict because subfields \"x\" conflict because \"a\" and \"b\" are different fields and subfields \"y\" conflict because \"c\" and \"d\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn ignores_unknown_fragments() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    let errors = test_operation_without_schema(
        "{
          field
          ...Unknown
          ...Known
        }
        fragment Known on T {
          field
          ...OtherUnknown
        }}",
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
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
