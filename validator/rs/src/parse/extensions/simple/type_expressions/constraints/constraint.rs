use crate::parse::extensions::simple::type_expressions::metavars;
use crate::parse::extensions::simple::type_expressions::metavalues;
use crate::parse::extensions::simple::type_expressions::constraints;

/// The types of constraints that can be imposed on metavariables.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstraintType {
    /// The value must be contained in this set.
    Within(metavalues::set::Set),

    /// The value must equal the return value of the given function.
    Function(constraints::function::Function, Vec<metavars::reference::Reference>),

    /// The value must match the given data type pattern.
    Pattern(metavalues::data_type::Pattern),
}

/// A constraint on a metavariable.
#[derive(Clone, Debug, PartialEq)]
pub struct Constraint {
    /// The data for the constraint.
    pub data: ConstraintType,

    /// A human-readable reason for the existence of the constraint, used for
    /// error messages when there are conflicting constraints.
    pub reason: String,
}
