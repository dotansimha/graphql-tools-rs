use crate::validation::validate::ValidationPlan;

use super::{
    FieldsOnCorrectType, FragmentsOnCompositeTypes, KnownArgumentNames, KnownFragmentNames,
    KnownTypeNames, LeafFieldSelections, LoneAnonymousOperation, NoFragmentsCycle,
    NoUndefinedVariables, NoUnusedFragments, NoUnusedVariables, OverlappingFieldsCanBeMerged,
    PossibleFragmentSpreads, ProvidedRequiredArguments, SingleFieldSubscriptions,
    UniqueArgumentNames, UniqueFragmentNames, UniqueOperationNames, UniqueVariableNames,
    VariablesAreInputTypes,
};

pub fn default_rules_validation_plan() -> ValidationPlan {
    let mut plan = ValidationPlan { rules: vec![] };

    plan.add_rule(Box::new(UniqueOperationNames {}));
    plan.add_rule(Box::new(LoneAnonymousOperation {}));
    plan.add_rule(Box::new(SingleFieldSubscriptions {}));
    plan.add_rule(Box::new(KnownTypeNames {}));
    plan.add_rule(Box::new(FragmentsOnCompositeTypes {}));
    plan.add_rule(Box::new(VariablesAreInputTypes {}));
    plan.add_rule(Box::new(LeafFieldSelections {}));
    plan.add_rule(Box::new(FieldsOnCorrectType {}));
    plan.add_rule(Box::new(UniqueFragmentNames {}));
    plan.add_rule(Box::new(KnownFragmentNames {}));
    plan.add_rule(Box::new(NoUnusedFragments {}));
    plan.add_rule(Box::new(OverlappingFieldsCanBeMerged {}));
    plan.add_rule(Box::new(NoFragmentsCycle {}));
    plan.add_rule(Box::new(PossibleFragmentSpreads {}));
    plan.add_rule(Box::new(NoUnusedVariables {}));
    plan.add_rule(Box::new(NoUndefinedVariables {}));
    plan.add_rule(Box::new(KnownArgumentNames {}));
    plan.add_rule(Box::new(UniqueArgumentNames {}));
    plan.add_rule(Box::new(UniqueVariableNames {}));
    plan.add_rule(Box::new(ProvidedRequiredArguments {}));

    plan
}
