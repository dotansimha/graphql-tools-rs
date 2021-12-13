use super::ValidationRule;
use crate::static_graphql::query::*;
use crate::validation::utils::ValidationError;
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

struct FindOverlappingFieldsThatCanBeMerged<'a> {
    discoverd_fields: HashMap<String, Field>,
    ctx: &'a mut ValidationContext,
}

impl<'a> FindOverlappingFieldsThatCanBeMerged<'a> {
    fn store_finding(&mut self, field: &Field, parent_type_name: Option<String>) {
      let base_field_name = field.alias.as_ref().unwrap_or(&field.name).clone();
      let field_identifier = match parent_type_name {
        Some(ref type_name) => format!("{}.{}", type_name, base_field_name.clone()),
        None => base_field_name.clone()
      };

        if let Some(existing) = self.discoverd_fields.get(&field_identifier) {
            if !existing.name.eq(&field.name) {
              self.ctx.report_error(ValidationError {
                  locations: vec![field.position, existing.position],
                  message: format!(
                      "Fields \"{}\" conflict because \"{}\" and \"{}\" are different fields. Use different aliases on the fields to fetch both if this was intentional.",
                      base_field_name, existing.name, field.name 
                  ),
              })
            }

            if existing.arguments.len() != field.arguments.len() {
              self.ctx.report_error(ValidationError {
                locations: vec![field.position, existing.position],
                message: format!(
                    "Fields \"{}\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional.",
                    field.name
                ),
              });
            } else {
              for (arg_name, arg_value) in &existing.arguments {
                  let arg_record_in_new_field = field
                      .arguments
                      .to_owned()
                      .into_iter()
                      .find(|(arg_name_in_new_field, _)| arg_name_in_new_field == arg_name);

                  match arg_record_in_new_field {
                      Some((_other_name, other_value)) if other_value.eq(arg_value) => {}
                      _ => {
                        self.ctx.report_error(ValidationError {
                          locations: vec![field.position, existing.position],
                          message: format!(
                              "Fields \"{}\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional.",
                              field.name
                          ),
                        });
                      }
                  }
              }
            }
        } else {
            self.discoverd_fields.insert(field_identifier, field.clone());
        }
    }

    pub fn find_in_selection_set(&mut self, selection_set: &SelectionSet, parent_type_name: Option<String>) {
        for selection in &selection_set.items {
            match selection {
                Selection::Field(field) => self.store_finding(field, parent_type_name.to_owned()),
                Selection::InlineFragment(inline_fragment) => {
                  match inline_fragment.type_condition {
                    Some(TypeCondition::On(ref type_condition)) => self.find_in_selection_set(&inline_fragment.selection_set, Some(type_condition.clone())),
                    _ => self.find_in_selection_set(&inline_fragment.selection_set, None),
                  }
                }

                Selection::FragmentSpread(fragment_spread) => {
                    if let Some(fragment) = self
                        .ctx
                        .fragments
                        .get(&fragment_spread.fragment_name)
                        .cloned()
                    {
                      match fragment.type_condition {
                        TypeCondition::On(type_condition) => self.find_in_selection_set(&fragment.selection_set, Some(type_condition.clone())),
                      }
                    }
                }
            }
        }
    }
}

impl QueryVisitor<ValidationContext> for OverlappingFieldsCanBeMerged {
    fn enter_selection_set(&self, node: &SelectionSet, ctx: &mut ValidationContext) {
        let mut finder = FindOverlappingFieldsThatCanBeMerged {
            discoverd_fields: HashMap::new(),
            ctx,
        };

        finder.find_in_selection_set(&node, None);
    }
}

impl ValidationRule for OverlappingFieldsCanBeMerged {
    fn validate(&self, ctx: &mut ValidationContext) {
        self.visit_document(&ctx.operation.clone(), ctx)
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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
        }"
        ,
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

