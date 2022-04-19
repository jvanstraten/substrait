use crate::parse::extensions::simple::type_expressions::constraints;
use crate::parse::extensions::simple::type_expressions::metavars;
use crate::parse::extensions::simple::type_expressions::metavalues;
use crate::output::diagnostic;
use std::rc::Rc;

/// A reference to a metavariable.
#[derive(Clone, Debug)]
pub struct Reference {
    /// The method through which the metavariable is referenced.
    key: metavars::key::Key,

    /// The raw parsed string that the user used to refer to the metavariable,
    /// if any. Used to keep track of the case/syntax convention that the user
    /// used, in order to produce better diagnostic messages. bind() moves this
    /// into the alias block.
    description: Option<Rc<String>>,

    /// Reference to the alias block for this metavariable. Initialized via
    /// bind().
    alias: Option<metavars::alias::Reference>,
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Try to print the description from the alias block.
        if let Some(alias) = &self.alias {
            if let Ok(alias) = alias.try_borrow() {
                return write!(f, "{alias}");
            }
        }
        
        // If we aren't bound to an alias block yet, or if we can't borrow
        // to access the description, see if we have a description of our own.
        if let Some(s) =  &self.description {
            return write!(f, "{s}");
        }

        // Fall back to the generated description of the key.
        self.key.fmt(f)
    }
}

impl PartialEq for Reference {
    /// Checks whether two references are functionally the same. Two references
    /// that alias the same value are NOT considered to be equal.
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Reference {
    /// Creates an (unbound) named reference to a metavariable.
    pub fn new_generic<S: ToString>(name: S) -> Self {
        let name = name.to_string();
        let key = name.to_ascii_lowercase();
        Reference {
            key: metavars::key::Key::Generic(key),
            description: Some(Rc::new(name)),
            alias: None,
        }
    }

    /// Creates a reference to a new, unique metavariable. The reference can be
    /// copied to refer to the same metavariable multiple times.
    pub fn new_inferred<S: ToString>(description: Option<S>) -> Self {
        Reference {
            key: metavars::key::Key::Inferred(metavars::key::Unique::default()),
            description: description.map(|x| Rc::new(x.to_string())),
            alias: None,
        }
    }

    /// Creates a reference to the type of the index'th parameter of the
    /// function being solved for.
    pub fn new_function_parameter_type(index: usize) -> Self {
        Reference {
            key: metavars::key::Key::FunctionParameterType(index),
            description: None,
            alias: None,
        }
    }

    /// Creates a reference to the return type of the function being solved
    /// for.
    pub fn new_function_return_type() -> Self {
        Reference {
            key: metavars::key::Key::FunctionReturnType,
            description: None,
            alias: None,
        }
    }

    /// Bind this metavariable reference to the given context.
    pub fn bind(&mut self, context: &mut Context) {
        todo!()
    }

    /// Adds an equality constraint between this metavariable and the other
    /// metavariable. This essentially just merges their data blocks. Both
    /// references must have been bound.
    pub fn constrain_equal(&self, other: &Reference) {
        let a_alias = self.alias.as_ref().expect("attempt to constrain unbound metavariable reference");
        let b_alias = other.alias.as_ref().expect("attempt to constrain unbound metavariable reference");

        // If the references are equivalent, their values are already equal by
        // definition.
        if Rc::ptr_eq(&a_alias, &b_alias) {
            return;
        }

        // Borrow the alias blocks.
        let a_alias = a_alias.borrow();
        let b_alias = b_alias.borrow();

        // If the references refer to the same data block already, their
        // values are already equal by definition.
        if Rc::ptr_eq(&a_alias.data, &b_alias.data) {
            return;
        }

        // Borrow the data blocks mutably. We first clone the Rc so we can drop
        // the alias borrows; we need to do this, because we're about to borrow
        // them mutably to re-alias them to the combined data block.
        let a_data_ref = a_alias.data.clone();
        let b_data_ref = b_alias.data.clone();
        let mut a_data = a_data_ref.borrow_mut();

        // Drop the borrows to the alias blocks.
        drop(a_alias);
        drop(b_alias);

        // Merge data block b into a.
        a_data.merge_with(&b_data_ref);
    }

    /// Constrains the value of the referred variable. The constraint is only
    /// added if no equivalent constraint exists yet.
    pub fn constrain(&self, constraint: constraints::constraint::Constraint) -> diagnostic::Result<()> {
        let alias = self.alias.as_ref().expect("attempt to constrain unbound metavariable reference").borrow();
        let mut data = alias.data.borrow_mut();
        data.constrain(constraint)
    }

    /// If the set of possible values for this metavariable has been reduced to
    /// only one possibility, return it. Otherwise returns None.
    pub fn value(&self) -> Option<metavalues::value::Value> {
        self.alias.as_ref().and_then(|alias| {
            let alias = alias.borrow();
            let data = alias.data.borrow();
            data.value()
        })
    }

    /// Returns whether this metavalue still has the given value as a
    /// possibility.
    pub fn matches(&self, value: &metavalues::value::Value) -> bool {
        if let Some(alias) = &self.alias {
            let alias = alias.borrow();
            let data = alias.data.borrow();
            data.matches(value)
        } else {
            true
        }
    }
}
