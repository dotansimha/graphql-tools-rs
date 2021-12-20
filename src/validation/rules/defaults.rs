use crate::validation::validate::ValidationPlan;

use super::{
    ExecutableDefinitions, FragmentsOnCompositeTypes, KnownFragmentNamesRule,
    LoneAnonymousOperation, NoUnusedFragments, OverlappingFieldsCanBeMerged,
};

pub fn default_rules_validation_plan() -> ValidationPlan {
    let mut plan = ValidationPlan { rules: vec![] };

    plan.add_rule(Box::new(LoneAnonymousOperation {}));
    plan.add_rule(Box::new(KnownFragmentNamesRule {}));
    plan.add_rule(Box::new(FragmentsOnCompositeTypes {}));
    plan.add_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    plan.add_rule(Box::new(NoUnusedFragments {}));
    plan.add_rule(Box::new(ExecutableDefinitions {}));

    plan
}
