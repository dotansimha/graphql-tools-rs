use std::collections::HashMap;

use super::{AbstractTypeDefinitionExtension, TypeInfoRegistry};
use crate::ast::ext::{TypeDefinitionExtension, UnionTypeExtension};
use crate::static_graphql::{
    query::{self, Selection, TypeCondition},
    schema::{self, TypeDefinition},
};
pub fn collect_fields(
    selection_set: &query::SelectionSet,
    parent_type: &schema::TypeDefinition,
    known_fragments: &HashMap<String, query::FragmentDefinition>,
    type_info_registry: &TypeInfoRegistry,
) -> HashMap<String, Vec<query::Field>> {
    let mut map = HashMap::new();
    let mut visited_fragments_names: Vec<String> = Vec::new();

    collect_fields_inner(
        selection_set,
        parent_type,
        known_fragments,
        type_info_registry,
        &mut map,
        &mut visited_fragments_names,
    );

    map
}

fn does_fragment_condition_match<'a>(
    fragment_condition: &'a Option<TypeCondition>,
    current_selection_set_type: &'a TypeDefinition,
    type_info_registry: &'a TypeInfoRegistry<'a>,
) -> bool {
    if let Some(TypeCondition::On(type_name)) = fragment_condition {
        if let Some(conditional_type) = type_info_registry.type_by_name.get(type_name) {
            println!("conditional_type: {}", conditional_type.name());
            println!(
                "current_selection_set_type: {}",
                current_selection_set_type.name()
            );
            if conditional_type
                .name()
                .eq(&current_selection_set_type.name())
            {
                return true;
            }

            if conditional_type.is_abstract_type() {
                match conditional_type {
                    TypeDefinition::Interface(interface_type) => {
                        return interface_type.is_implemented_by(current_selection_set_type)
                    }
                    TypeDefinition::Union(union_type) => {
                        return union_type.has_sub_type(&current_selection_set_type.name())
                    }
                    _ => return false,
                }
            }
        }

        false
    } else {
        true
    }
}

fn collect_fields_inner(
    selection_set: &query::SelectionSet,
    parent_type: &schema::TypeDefinition,
    known_fragments: &HashMap<String, query::FragmentDefinition>,
    type_info_registry: &TypeInfoRegistry,
    result_arr: &mut HashMap<String, Vec<query::Field>>,
    visited_fragments_names: &mut Vec<String>,
) {
    println!("collect_fields_inner");
    selection_set.items.iter().for_each(|item| match item {
        Selection::Field(f) => {
            let existing = result_arr.entry(f.name.clone()).or_insert(vec![]);
            existing.push(f.clone());
        }
        Selection::InlineFragment(f) => {
            if does_fragment_condition_match(&f.type_condition, parent_type, type_info_registry) {
                collect_fields_inner(
                    &f.selection_set,
                    &parent_type,
                    known_fragments,
                    type_info_registry,
                    result_arr,
                    visited_fragments_names,
                );
            }
        }
        Selection::FragmentSpread(f) => {
            if visited_fragments_names
                .iter()
                .find(|name| f.fragment_name.eq(*name))
                .is_none()
            {
                visited_fragments_names.push(f.fragment_name.clone());

                if let Some(fragment) = known_fragments.get(&f.fragment_name) {
                    if does_fragment_condition_match(
                        &Some(fragment.type_condition.clone()),
                        &parent_type,
                        type_info_registry,
                    ) {
                        collect_fields_inner(
                            &fragment.selection_set,
                            &parent_type,
                            known_fragments,
                            type_info_registry,
                            result_arr,
                            visited_fragments_names,
                        );
                    }
                }
            }
        }
    });
}
