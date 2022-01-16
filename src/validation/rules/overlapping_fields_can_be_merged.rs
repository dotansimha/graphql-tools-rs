use graphql_parser::query::{Definition, TypeCondition};
use graphql_parser::Pos;

use super::ValidationRule;
use crate::ast::ext::TypeDefinitionExtension;
use crate::ast::{
    visit_document, FieldByNameExtension, OperationVisitor, OperationVisitorContext,
    SchemaDocumentExtension, TypeExtension, ValueExtension,
};
use crate::static_graphql::query::*;
use crate::static_graphql::schema::{
    Document as SchemaDocument, Field as FieldDefinition, TypeDefinition,
};
use crate::validation::utils::ValidationContext;
use crate::validation::utils::{ValidationError, ValidationErrorContext};
use std::borrow::Borrow;
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
pub struct OverlappingFieldsCanBeMerged {
    named_fragments: HashMap<String, FragmentDefinition>,
    compared_fragments: PairSet,
}

#[derive(Debug)]
struct Conflict(ConflictReason, Vec<Pos>, Vec<Pos>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConflictReason(String, ConflictReasonMessage);

#[derive(Debug)]
struct AstAndDef(Option<TypeDefinition>, Field, Option<FieldDefinition>);

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

impl OverlappingFieldsCanBeMerged {
    pub fn new() -> Self {
        Self {
            named_fragments: HashMap::new(),
            compared_fragments: PairSet::new(),
        }
    }

    fn find_conflicts_within_selection_set(
        &mut self,
        schema: &SchemaDocument,
        parent_type: Option<&TypeDefinition>,
        selection_set: &SelectionSet,
    ) -> Vec<Conflict> {
        let mut conflicts = Vec::<Conflict>::new();

        let (field_map, fragment_names) =
            self.get_fields_and_fragment_names(schema, parent_type, selection_set);

        self.collect_conflicts_within(schema, &mut conflicts, &field_map);

        for (i, frag_name1) in fragment_names.iter().enumerate() {
            self.collect_conflicts_between_fields_and_fragment(
                schema,
                &mut conflicts,
                &field_map,
                frag_name1,
                false,
            );

            for frag_name2 in &fragment_names[i + 1..] {
                self.collect_conflicts_between_fragments(
                    schema,
                    &mut conflicts,
                    frag_name1,
                    frag_name2,
                    false,
                );
            }
        }

        conflicts
    }

    fn collect_conflicts_within(
        &mut self,
        schema: &SchemaDocument,
        conflicts: &mut Vec<Conflict>,
        field_map: &OrderedMap<String, Vec<AstAndDef>>,
    ) {
        for (out_field_name, fields) in field_map.iter() {
            for (index, first) in fields.iter().enumerate() {
                for second in &fields[index + 1..] {
                    if let Some(conflict) =
                        self.is_conflicting(schema, out_field_name, first, second, false)
                    {
                        conflicts.push(conflict)
                    }
                }
            }
        }
    }

    fn is_same_arguments(
        &self,
        f1_args: &Vec<(String, Value)>,
        f2_args: &Vec<(String, Value)>,
    ) -> bool {
        if f1_args.len() != f2_args.len() {
            return false;
        }

        f1_args.iter().all(|&(ref n1, ref v1)| {
            if let Some(&(_, ref v2)) = f2_args.iter().find(|&&(ref n2, _)| n1.eq(n2)) {
                v1.compare(&v2)
            } else {
                false
            }
        })
    }

    fn is_type_conflict(&self, schema: &SchemaDocument, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (Type::ListType(t1), Type::ListType(t2)) => self.is_type_conflict(schema, t1, t2),
            (Type::NonNullType(t1), Type::NonNullType(t2)) => self.is_type_conflict(schema, t1, t2),
            (Type::NamedType(t1), Type::NamedType(t2)) => {
                let schema_type1 = schema.type_by_name(t1);
                let schema_type2 = schema.type_by_name(t2);

                if schema_type1.map(|t| t.is_leaf_type()).unwrap_or(false)
                    || schema_type2.map(|t| t.is_leaf_type()).unwrap_or(false)
                {
                    t1.ne(t2)
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    fn is_conflicting(
        &mut self,
        schema: &SchemaDocument,
        out_field_name: &String,
        first: &AstAndDef,
        second: &AstAndDef,
        parents_mutually_exclusive: bool,
    ) -> Option<Conflict> {
        let AstAndDef(ref parent_type1, ref field1, ref field1_def) = *first;
        let AstAndDef(ref parent_type2, ref field2, ref field2_def) = *second;

        let mutually_exclusive = parents_mutually_exclusive
            || (parent_type1.name().ne(&parent_type2.name())
                && parent_type1.is_object_type()
                && parent_type2.is_object_type());

        if !mutually_exclusive {
            let name1 = &field1.name;
            let name2 = &field2.name;

            if name1 != name2 {
                return Some(Conflict(
                    ConflictReason(
                        out_field_name.clone(),
                        ConflictReasonMessage::Message(format!(
                            "\"{}\" and \"{}\" are different fields",
                            name1, name2
                        )),
                    ),
                    vec![field1.position],
                    vec![field2.position],
                ));
            }

            if !self.is_same_arguments(&field1.arguments, &field2.arguments) {
                return Some(Conflict(
                    ConflictReason(
                        out_field_name.clone(),
                        ConflictReasonMessage::Message("they have differing arguments".to_string()),
                    ),
                    vec![field1.position],
                    vec![field2.position],
                ));
            }
        }

        let t1 = field1_def.as_ref().map(|def| &def.field_type);
        let t2 = field2_def.as_ref().map(|def| &def.field_type);

        if let (Some(t1), Some(t2)) = (t1, t2) {
            if self.is_type_conflict(schema, t1, t2) {
                return Some(Conflict(
                    ConflictReason(
                        out_field_name.to_owned(),
                        ConflictReasonMessage::Message(format!(
                            "they return conflicting types \"{}\" and \"{}\"",
                            t1, t2
                        )),
                    ),
                    vec![field1.position],
                    vec![field2.position],
                ));
            }
        }

        let conflicts = self.find_conflicts_between_sub_selection_sets(
            schema,
            mutually_exclusive,
            t1.map(|v| v.inner_type()),
            &field1.selection_set,
            t2.map(|v| v.inner_type()),
            &field2.selection_set,
        );

        return self.subfield_conflicts(
            &conflicts,
            out_field_name,
            field1.position,
            field1.position,
        );
    }

    fn subfield_conflicts(
        &self,
        conflicts: &Vec<Conflict>,
        out_field_name: &String,
        f1_pos: Pos,
        f2_pos: Pos,
    ) -> Option<Conflict> {
        if conflicts.is_empty() {
            return None;
        }

        Some(Conflict(
            ConflictReason(
                out_field_name.clone(),
                ConflictReasonMessage::Nested(conflicts.iter().map(|v| v.0.clone()).collect()),
            ),
            vec![f1_pos]
                .into_iter()
                .chain(conflicts.iter().flat_map(|v| v.1.clone()))
                .collect(),
            vec![f2_pos]
                .into_iter()
                .chain(conflicts.iter().flat_map(|v| v.1.clone()))
                .collect(),
        ))
    }

    fn find_conflicts_between_sub_selection_sets(
        &mut self,
        schema: &SchemaDocument,
        mutually_exclusive: bool,
        parent_type_name1: Option<String>,
        selection_set1: &SelectionSet,
        parent_type_name2: Option<String>,
        selection_set2: &SelectionSet,
    ) -> Vec<Conflict> {
        let mut conflicts = Vec::<Conflict>::new();
        let parent_type1 = parent_type_name1.and_then(|t| schema.type_by_name(&t));
        let parent_type2 = parent_type_name2.and_then(|t| schema.type_by_name(&t));

        let (field_map1, fragment_names1) =
            self.get_fields_and_fragment_names(schema, parent_type1.as_ref(), selection_set1);
        let (field_map2, fragment_names2) =
            self.get_fields_and_fragment_names(schema, parent_type2.as_ref(), selection_set2);

        self.collect_conflicts_between(
            schema,
            &mut conflicts,
            mutually_exclusive,
            &field_map1,
            &field_map2,
        );

        for fragment_name in &fragment_names2 {
            self.collect_conflicts_between_fields_and_fragment(
                schema,
                &mut conflicts,
                &field_map1,
                fragment_name,
                mutually_exclusive,
            );
        }

        for fragment_name in &fragment_names1 {
            self.collect_conflicts_between_fields_and_fragment(
                schema,
                &mut conflicts,
                &field_map2,
                fragment_name,
                mutually_exclusive,
            );
        }

        for fragment_name1 in &fragment_names1 {
            for fragment_name2 in &fragment_names2 {
                self.collect_conflicts_between_fragments(
                    schema,
                    &mut conflicts,
                    fragment_name1,
                    fragment_name2,
                    mutually_exclusive,
                );
            }
        }

        conflicts
    }

    fn collect_conflicts_between_fields_and_fragment(
        &mut self,
        schema: &SchemaDocument,
        conflicts: &mut Vec<Conflict>,
        field_map: &OrderedMap<String, Vec<AstAndDef>>,
        fragment_name: &String,
        mutually_exclusive: bool,
    ) {
        let fragment = match self.named_fragments.get(fragment_name) {
            Some(f) => f,
            None => return,
        };

        let (field_map2, fragment_names2) =
            self.get_referenced_fields_and_fragment_names(schema, fragment);

        self.collect_conflicts_between(
            schema,
            conflicts,
            mutually_exclusive,
            field_map,
            &field_map2,
        );

        for fragment_name2 in &fragment_names2 {
            self.collect_conflicts_between_fields_and_fragment(
                schema,
                conflicts,
                field_map,
                fragment_name2,
                mutually_exclusive,
            );
        }
    }

    fn collect_conflicts_between_fragments(
        &mut self,
        schema: &SchemaDocument,
        conflicts: &mut Vec<Conflict>,
        fragment_name1: &String,
        fragment_name2: &String,
        mutually_exclusive: bool,
    ) {
        if fragment_name1.eq(fragment_name2) {
            return;
        }

        let fragment1 = match self.named_fragments.get(fragment_name1) {
            Some(f) => f,
            None => return,
        };

        let fragment2 = match self.named_fragments.get(fragment_name2) {
            Some(f) => f,
            None => return,
        };

        {
            if self.compared_fragments.borrow().contains(
                fragment_name1,
                fragment_name2,
                mutually_exclusive,
            ) {
                return;
            }
        }

        {
            self.compared_fragments
                .insert(fragment_name1, fragment_name2, mutually_exclusive);
        }

        let (field_map1, fragment_names1) =
            self.get_referenced_fields_and_fragment_names(schema, fragment1);
        let (field_map2, fragment_names2) =
            self.get_referenced_fields_and_fragment_names(schema, fragment2);

        self.collect_conflicts_between(
            schema,
            conflicts,
            mutually_exclusive,
            &field_map1,
            &field_map2,
        );

        for fragment_name2 in &fragment_names2 {
            self.collect_conflicts_between_fragments(
                schema,
                conflicts,
                fragment_name1,
                fragment_name2,
                mutually_exclusive,
            );
        }

        for fragment_name1 in &fragment_names1 {
            self.collect_conflicts_between_fragments(
                schema,
                conflicts,
                fragment_name1,
                fragment_name2,
                mutually_exclusive,
            );
        }
    }

    fn get_referenced_fields_and_fragment_names(
        &self,
        schema: &SchemaDocument,
        fragment: &FragmentDefinition,
    ) -> (OrderedMap<String, Vec<AstAndDef>>, Vec<String>) {
        let TypeCondition::On(type_condition) = &fragment.type_condition;
        let fragment_type = schema.type_by_name(type_condition);

        self.get_fields_and_fragment_names(schema, fragment_type.as_ref(), &fragment.selection_set)
    }

    fn collect_conflicts_between(
        &mut self,
        schema: &SchemaDocument,
        conflicts: &mut Vec<Conflict>,
        mutually_exclusive: bool,
        field_map1: &OrderedMap<String, Vec<AstAndDef>>,
        field_map2: &OrderedMap<String, Vec<AstAndDef>>,
    ) {
        for (response_name, fields1) in field_map1.iter() {
            if let Some(fields2) = field_map2.get(response_name) {
                for field1 in fields1 {
                    for field2 in fields2 {
                        if let Some(conflict) = self.is_conflicting(
                            schema,
                            response_name,
                            field1,
                            field2,
                            mutually_exclusive,
                        ) {
                            conflicts.push(conflict);
                        }
                    }
                }
            }
        }
    }

    fn get_fields_and_fragment_names(
        &self,
        schema: &SchemaDocument,
        parent_type: Option<&TypeDefinition>,
        selection_set: &SelectionSet,
    ) -> (OrderedMap<String, Vec<AstAndDef>>, Vec<String>) {
        let mut ast_and_defs = OrderedMap::<String, Vec<AstAndDef>>::new();
        let mut fragment_names = Vec::<String>::new();

        self.collect_fields_and_fragment_names(
            schema,
            parent_type,
            selection_set,
            &mut ast_and_defs,
            &mut fragment_names,
        );

        (ast_and_defs, fragment_names)
    }

    fn collect_fields_and_fragment_names(
        &self,
        schema: &SchemaDocument,
        parent_type: Option<&TypeDefinition>,
        selection_set: &SelectionSet,
        ast_and_defs: &mut OrderedMap<String, Vec<AstAndDef>>,
        fragment_names: &mut Vec<String>,
    ) {
        for selection in &selection_set.items {
            match selection {
                Selection::Field(field) => {
                    let field_name = &field.name;
                    let field_def = parent_type.and_then(|t| t.field_by_name(field_name));
                    let out_field_name = field.alias.as_ref().unwrap_or(field_name);

                    if !ast_and_defs.contains_key(out_field_name) {
                        ast_and_defs.insert(out_field_name.clone(), Vec::new());
                    }

                    ast_and_defs
                        .get_mut(out_field_name)
                        .unwrap()
                        .push(AstAndDef(parent_type.cloned(), field.clone(), field_def));
                }
                Selection::FragmentSpread(fragment_spread) => {
                    if !fragment_names
                        .iter()
                        .any(|n| n.eq(&fragment_spread.fragment_name))
                    {
                        fragment_names.push(fragment_spread.fragment_name.clone());
                    }
                }
                Selection::InlineFragment(inline_fragment) => {
                    let fragment_type = inline_fragment
                        .type_condition
                        .as_ref()
                        .and_then(|type_condition| {
                            let TypeCondition::On(type_condition) = type_condition;

                            schema.type_by_name(type_condition)
                        })
                        .or(parent_type.cloned());

                    self.collect_fields_and_fragment_names(
                        schema,
                        fragment_type.as_ref(),
                        &inline_fragment.selection_set,
                        ast_and_defs,
                        fragment_names,
                    )
                }
            }
        }
    }
}

impl<'a> OperationVisitor<'a, ValidationErrorContext> for OverlappingFieldsCanBeMerged {
    fn enter_document(
        &mut self,
        _visitor_context: &mut OperationVisitorContext<ValidationErrorContext>,
        document: &Document,
    ) {
        for definition in &document.definitions {
            if let Definition::Fragment(fragment) = definition {
                self.named_fragments
                    .insert(fragment.name.clone(), fragment.clone());
            }
        }
    }

    fn enter_selection_set(
        &mut self,
        visitor_context: &mut OperationVisitorContext<ValidationErrorContext>,
        selection_set: &SelectionSet,
    ) {
        let parent_type = visitor_context.current_parent_type();
        let schema = visitor_context.schema;
        let found_conflicts =
            self.find_conflicts_within_selection_set(&schema, parent_type, selection_set);

        for Conflict(ConflictReason(reason_name, reason_msg), mut p1, p2) in found_conflicts {
            p1.extend(p2);

            visitor_context.user_context.report_error(ValidationError {
                message: error_message(&reason_name, &reason_msg),
                locations: p1,
            });
        }
    }
}

fn error_message(reason_name: &str, reason: &ConflictReasonMessage) -> String {
    let suffix = "Use different aliases on the fields to fetch both if this was intentional.";

    format!(
        r#"Fields "{}" conflict because {}. {}"#,
        reason_name,
        format_reason(reason),
        suffix
    )
}

fn format_reason(reason: &ConflictReasonMessage) -> String {
    match *reason {
        ConflictReasonMessage::Message(ref name) => name.clone(),
        ConflictReasonMessage::Nested(ref nested) => nested
            .iter()
            .map(|&ConflictReason(ref name, ref subreason)| {
                format!(
                    r#"subfields "{}" conflict because {}"#,
                    name,
                    format_reason(subreason)
                )
            })
            .collect::<Vec<_>>()
            .join(" and "),
    }
}

impl ValidationRule for OverlappingFieldsCanBeMerged {
    fn validate<'a>(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut helper = ValidationErrorContext::new();

        visit_document(
            &mut OverlappingFieldsCanBeMerged::new(),
            &ctx.operation,
            &mut OperationVisitorContext::new(&mut helper, &ctx.operation, &ctx.schema),
        );

        helper.errors
    }
}

#[test]
fn unique_fields() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment uniqueFields on Dog {
          name
          nickname
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn identical_fields() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment mergeIdenticalFields on Dog {
          name
          name
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn identical_fields_and_identical_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment mergeIdenticalFieldsWithIdenticalArgs on Dog {
          doesKnowCommand(dogCommand: SIT)
          doesKnowCommand(dogCommand: SIT)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn identical_fields_and_identical_directives() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment mergeSameFieldsWithSameDirectives on Dog {
          name @include(if: true)
          name @include(if: true)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn different_args_different_aliases() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment differentArgsWithDifferentAliases on Dog {
          knowsSit: doesKnowCommand(dogCommand: SIT)
          knowsDown: doesKnowCommand(dogCommand: DOWN)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn different_directives_different_aliases() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment differentDirectivesWithDifferentAliases on Dog {
          nameIfTrue: name @include(if: true)
          nameIfFalse: name @include(if: false)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn different_skip_include_directives() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment differentDirectivesWithDifferentAliases on Dog {
          name @include(if: true)
          name @include(if: false)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    assert_eq!(get_messages(&errors).len(), 0);
}

#[test]
fn same_alias_different_field_target() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment sameAliasesWithDifferentFieldTargets on Dog {
          fido: name
          fido: nickname
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"fido\" conflict because \"name\" and \"nickname\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn same_alias_non_overlapping_field_target() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment sameAliasesWithDifferentFieldTargets on Pet {
          ... on Dog {
            name
          }
          ... on Cat {
            name: nickname
          }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn alias_masking_direct_access() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment aliasMaskingDirectFieldAccess on Dog {
          name: nickname
          name
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"name\" conflict because \"nickname\" and \"name\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn different_args_second_adds() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment conflictingArgs on Dog {
          doesKnowCommand
          doesKnowCommand(dogCommand: HEEL)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"doesKnowCommand\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn different_args_declared_on_first() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment conflictingArgs on Dog {
          doesKnowCommand(dogCommand: SIT)
          doesKnowCommand
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"doesKnowCommand\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn different_arg_values() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment conflictingArgs on Dog {
          doesKnowCommand(dogCommand: SIT)
          doesKnowCommand(dogCommand: HEEL)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"doesKnowCommand\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn conflicting_arg_names() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment conflictingArgs on Dog {
          isAtLocation(x: 0)
          isAtLocation(y: 0)
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"isAtLocation\" conflict because they have differing arguments. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn allow_different_args_when_possible_with_different_args() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment conflictingArgs on Pet {
          ... on Dog {
            name(surname: true)
          }
          ... on Cat {
            name
          }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn conflict_in_fragment_spread() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
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
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"x\" conflict because \"a\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn deep_conflict() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "{
          field {
            x: a
          }
          field {
            x: b
          }
        }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec!["Fields \"field\" conflict because subfields \"x\" conflict because \"a\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional."]);
}

#[test]
fn report_each_conflict_once() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
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
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 3);
    assert_eq!(messages, vec![
      "Fields \"x\" conflict because \"a\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional.",
      "Fields \"x\" conflict because \"c\" and \"a\" are different fields. Use different aliases on the fields to fetch both if this was intentional.",
      "Fields \"x\" conflict because \"c\" and \"b\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
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

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment fragA on Human { name, relatives { name, ...fragA } }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn does_not_infinite_loop_on_immediately_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "fragment fragA on Human { name, ...fragA }",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn does_not_infinite_loop_on_transitively_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "
        fragment fragA on Human { name, ...fragB }
        fragment fragB on Human { name, ...fragC }
        fragment fragC on Human { name, ...fragA }
      ",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 0);
}

#[test]
fn finds_invalid_case_even_with_immediately_recursive_fragment() {
    use crate::validation::test_utils::*;

    let mut plan = create_plan_from_rule(Box::new(OverlappingFieldsCanBeMerged::new()));
    let errors = test_operation_with_schema(
        "
        fragment sameAliasesWithDifferentFieldTargets on Dog {
          ...sameAliasesWithDifferentFieldTargets
          fido: name
          fido: nickname
        }
      ",
        TEST_SCHEMA,
        &mut plan,
    );

    let messages = get_messages(&errors);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages, vec![
      "Fields \"fido\" conflict because \"name\" and \"nickname\" are different fields. Use different aliases on the fields to fetch both if this was intentional."
    ]);
}
