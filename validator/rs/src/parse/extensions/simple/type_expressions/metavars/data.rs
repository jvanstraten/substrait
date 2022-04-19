use crate::parse::extensions::simple::type_expressions::constraints;
use crate::parse::extensions::simple::type_expressions::metavars;
use crate::parse::extensions::simple::type_expressions::metavalues;
use crate::output::diagnostic;
use std::rc::Rc;
use std::cell::RefCell;

/// A data block for a metavariable. This holds the set of constraints imposed
/// on the variable, and caches the possible values that the variable may still
/// have.
#[derive(Clone, Debug)]
pub struct Data {
    /// Weak references to all aliases that refer to this data block. For
    /// example, in fn(T) -> T, the return type, the first parameter, and
    /// generic T all refer to the same data block.
    aliases: Vec<metavars::alias::Weak>,

    /// The constraints on the value of this metavariable.
    constraints: Vec<constraints::constraint::Constraint>,

    /// The possible values remaining for this metavariable.
    values: metavalues::set::Set,

    /// Whether changes have been made to this data block since the last poll.
    updated: bool,
}

impl Data {
    /// Merges this data block with the other data block. All references to
    /// the other data block will be redirected to this data block.
    pub fn merge_with(&mut self, other: &Reference) {
        let other_data = other.borrow_mut();

        // Copy stuff from the data block for b to the data block for a, such
        // that a becomes the combined data block for both. Remap aliases to
        // block b to block a instead, dropping expired weak references while
        // we're at it.
        self.aliases.extend(other_data.aliases.drain(..).filter(|x| {
            x.upgrade().map(|x| x.borrow_mut().data = other.clone()).is_some()
        }));
        for constraint in other_data.constraints.drain(..) {
            self.constrain(constraint);
        }
    }

    /// Further constrains the value. The constraint is only added if no
    /// equivalent constraint exists yet.
    pub fn constrain(&mut self, constraint: constraints::constraint::Constraint) -> diagnostic::Result<()> {
        if !self.constraints.iter().any(|x| x == &constraint) {
            self.values.constrain(&constraint);
            self.constraints.push(constraint);
            if self.values.is_empty() {
                todo!()
            }
            self.updated = true;
        }
        Ok(())
    }

    /// If the set of possible values for this metavariable has been reduced to
    /// only one possibility, return it. Otherwise returns None.
    pub fn value(&self) -> Option<metavalues::value::Value> {
        self.values.value()
    }

    /// Returns whether this metavalue still has the given value as a
    /// possibility.
    pub fn matches(&self, value: &metavalues::value::Value) -> bool {
        self.values.contains(value)
    }

    /// Returns whether there were any updates to the constraints on this
    /// metavariable since the last check.
    pub fn check_updates(&mut self) -> bool {
        if self.updated {
            self.updated = false;
            true
        } else {
            false
        }
    }
}

/// Reference to the data block for a metavariable, holding its constraints
/// and remaining possible values.
pub type Reference = Rc<RefCell<Data>>;
