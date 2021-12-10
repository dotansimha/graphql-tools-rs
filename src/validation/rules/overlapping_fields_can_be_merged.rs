use std::collections::HashMap;

use graphql_parser::query::Field;

use super::ValidationRule;
use crate::{ast::QueryVisitor, validation::utils::ValidationContext};

pub struct OverlappingFieldsCanBeMerged;

// struct FindOverlappingFieldsThatCanBeMerged<'a> {
//     discoverd_fields: HashMap<&'a String, &'a Field<'a, String>>,
//     ctx: &'a ValidationContext<'a>,
// }

// impl<'a> FindOverlappingFieldsThatCanBeMerged<'a> {
//     pub fn find_in_selection_set(
//         &mut self,
//         selection_set: &'a graphql_parser::query::SelectionSet<'a, String>,
//     ) {
//         for selection in &selection_set.items {
//             match selection {
//                 graphql_parser::query::Selection::Field(field) => {
//                     self.discoverd_fields
//                         .insert(field.alias.as_ref().unwrap_or(&field.name), field);
//                 }
//                 graphql_parser::query::Selection::InlineFragment(inline_fragment) => {
//                     self.find_in_selection_set(&inline_fragment.selection_set);
//                 }
//                 graphql_parser::query::Selection::FragmentSpread(fragment_spread) => {
//                     if let Some(fragment) = self.ctx.fragments.get(&fragment_spread.fragment_name) {
//                         self.find_in_selection_set(&fragment.selection_set);
//                     }
//                 }
//             }
//         }
//     }
// }

// impl<'a> QueryVisitor<'a, ValidationContext<'a>> for OverlappingFieldsCanBeMerged {
//     fn enter_selection_set(
//         &mut self,
//         node: &'a graphql_parser::query::SelectionSet<'a, String>,
//         ctx: &'a ValidationContext<'a>,
//     ) {
//         let mut finder = FindOverlappingFieldsThatCanBeMerged {
//             discoverd_fields: HashMap::new(),
//             ctx,
//         };

//         finder.find_in_selection_set(&node);
//     }
// }

impl<'a> ValidationRule<'a> for OverlappingFieldsCanBeMerged {
    fn validate(&mut self, ctx: &'a ValidationContext<'a>) {
        // self.visit_document(ctx.operation, ctx)
    }
}
