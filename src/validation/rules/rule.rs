use crate::{ast::OperationVisitorContext, validation::utils::ValidationErrorContext};

pub trait ValidationRule: Send + Sync {
    fn validate<'a>(
        &self,
        _ctx: &mut OperationVisitorContext<'a>,
        _error_collector: &mut ValidationErrorContext,
    ) -> ();
}
