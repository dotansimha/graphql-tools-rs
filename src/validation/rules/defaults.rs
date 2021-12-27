use crate::validation::validate::ValidationPlan;

use super::{
    FieldsOnCorrectType, FragmentsOnCompositeTypes, KnownFragmentNamesRule, LeafFieldSelections,
    LoneAnonymousOperation, NoUnusedFragments, OverlappingFieldsCanBeMerged, UniqueOperationNames,
};

pub fn default_rules_validation_plan() -> ValidationPlan {
    let mut plan = ValidationPlan { rules: vec![] };

    plan.add_rule(Box::new(LoneAnonymousOperation {}));
    plan.add_rule(Box::new(FieldsOnCorrectType {}));
    plan.add_rule(Box::new(KnownFragmentNamesRule {}));
    plan.add_rule(Box::new(FragmentsOnCompositeTypes {}));
    plan.add_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    plan.add_rule(Box::new(NoUnusedFragments {}));
    plan.add_rule(Box::new(LeafFieldSelections {}));
    plan.add_rule(Box::new(UniqueOperationNames {}));

    plan
}
