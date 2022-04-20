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

    /// Set when it shouldn't be possible for this value to be further
    /// constrained. If this is set, merge_with() will always panic, and
    /// constrain() will panic if a constraint is added that isn't already
    /// in the list. This flag is necessary to determine when solving is
    /// complete, and for determining when covers() can start returning a
    /// value.
    complete: bool,
}

impl Data {
    /// Merges this data block with the other data block. All references to
    /// the other data block will be redirected to this data block.
    pub fn merge_with(&mut self, other: &Reference) {
        assert!(!self.complete);
        assert!(!other.complete);
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
            assert!(!self.complete);
            self.values.constrain(&constraint);
            self.constraints.push(constraint);
            if self.values.is_empty() {
                // Determine a minimal subset of constraints that
                // overconstrains the variable (which must include at least the
                // new constraint) and generate an error message from that.
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

    /// Returns whether the value of this metavariable can be proven to either
    /// cover or not cover the value of the other metavariable, where
    /// "a covers b" means that all possible values of b are also possible
    /// values of a. If this cannot yet be proven, None is returned. This
    /// happens when:
    /// 
    ///  - self currently covers other, but new constraints may still be added
    ///    to self; or
    ///  - self currently does not cover other, but they do have at least one
    ///    possible value in common, and new constraints may still be added to
    ///    remove possibile values from other.
    pub fn covers(&self, other: &Data) -> Option<bool> {
        match self.values.superset_of(other.values) {
            Some(true) => {
                if self.complete {
                    Some(true)
                } else {
                    None
                }
            }
            Some(false) => {
                if other.complete || !self.values.intersects_with(other.values) {
                    Some(false)
                } else {
                    None
                }
            }
            None => None,
        }
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

    /// Marks this variable as being fully constrained, i.e. no further
    /// constraints will be imposed. Any covers() function evaluation that
    /// relies on this fact may start returning a value.
    pub fn mark_complete(&mut self) {
        self.complete = true;
    }

    /// Returns whether this value has been completely constrained.
    pub fn is_complete(&self) -> bool {
        self.complete
    }
}

/// Reference to the data block for a metavariable, holding its constraints
/// and remaining possible values.
pub type Reference = Rc<RefCell<Data>>;
