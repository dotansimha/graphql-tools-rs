use crate::validation::validate::ValidationPlan;

use super::{
    FragmentsOnCompositeTypes, KnownFragmentNamesRule, LoneAnonymousOperation, NoUnusedFragments,
    OverlappingFieldsCanBeMerged,
};

pub fn default_rules_validation_plan<'a>() -> ValidationPlan<'a> {
    let mut plan = ValidationPlan { rules: vec![] };

    plan.add_rule(Box::new(LoneAnonymousOperation {}));
    plan.add_rule(Box::new(KnownFragmentNamesRule {}));
    plan.add_rule(Box::new(FragmentsOnCompositeTypes {}));
    plan.add_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    plan.add_rule(Box::new(NoUnusedFragments {}));

    plan
}
