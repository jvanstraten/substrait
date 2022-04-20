use crate::parse::extensions::simple::type_expressions::metavalues;
use crate::parse::extensions::simple::type_expressions::constraints;
use crate::util;
use crate::output::data_type;
use crate::output::diagnostic;
use std::sync::Arc;

/// A set of boolean metavalues.
#[derive(Clone, Debug, PartialEq)]
pub enum Boolean {
    /// The set contains both true and false.
    All,

    /// The set contains only the given value.
    Some(bool),

    /// The set is empty.
    None,
}

impl Boolean {
    /// Returns the set containing all possible values.
    pub fn full() -> Self {
        Boolean::All
    }

    /// Returns the empty set.
    pub fn empty() -> Self {
        Boolean::None
    }

    /// Remove all values in the set that do not satisfy the given constraint.
    pub fn constrain(&mut self, constraint: &constraints::constraint::Constraint) {
        todo!()
    }

    /// Returns whether the set contains the given value.
    pub fn contains(&self, value: bool) -> bool {
        match self {
            Boolean::All => true,
            Boolean::Some(x) => x == &value,
            Boolean::None => false,
        }
    }

    /// Returns whether this is a superset of other.
    pub fn superset_of(&self, other: &Boolean) -> bool {
        match (self, other) {
            (Boolean::All, _) => true,
            (_, Boolean::All) => false,
            (_, Boolean::None) => true,
            (Boolean::None, _) => false,
            (Boolean::Some(x), Boolean::Some(y)) => x == y,
        }
    }

    /// Returns whether this set intersects with the other.
    pub fn intersects_with(&self, other: &Boolean) -> bool {
        match (self, other) {
            (Boolean::None, _) => false,
            (_, Boolean::None) => false,
            (Boolean::All, _) => true,
            (_, Boolean::All) => true,
            (Boolean::Some(x), Boolean::Some(y)) => x == y,
        }
    }

    /// Returns whether this is the empty set.
    pub fn is_empty(&self) -> bool {
        matches!(self, Boolean::None)
    }

    /// If this set contains exactly one value, return it.
    pub fn value(&self) -> Option<bool> {
        if let Boolean::Some(x) = self {
            Some(*x)
        } else {
            None
        }
    }
}

/// A set of integer metavalues.
#[derive(Clone, Debug, PartialEq)]
pub struct Integer(util::integer_set::IntegerSet<i64>);

impl Integer {
    /// Returns the set containing all possible values.
    pub fn full() -> Self {
        Self(util::integer_set::IntegerSet::new_full())
    }

    /// Returns the empty set.
    pub fn empty() -> Self {
        Self(util::integer_set::IntegerSet::default())
    }

    /// Remove all values in the set that do not satisfy the given constraint.
    pub fn constrain(&mut self, constraint: &constraints::constraint::Constraint) {
        todo!()
    }

    /// Returns whether the set contains the given value.
    pub fn contains(&self, value: i64) -> bool {
        self.0.contains(&value)
    }

    /// Returns whether this is a superset of other.
    pub fn superset_of(&self, other: &Integer) -> bool {
        other.0.subtract(&self.0).is_empty()
    }

    /// Returns whether this set intersects with the other.
    pub fn intersects_with(&self, other: &Integer) -> bool {
        !self.0.intersect(&other.0).is_empty()
    }

    /// Returns whether this is the empty set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// If this set contains exactly one value, return it.
    pub fn value(&self) -> Option<i64> {
        self.0.value()
    }
}

/// A set of data type metavalues.
#[derive(Clone, Debug, PartialEq)]
pub enum DataType {
    /// The set contains all data types.
    All,

    /// The set consists of all data types matched by at least one of these
    /// patterns.
    Some(Vec<metavalues::data_type::Pattern>),
}

impl DataType {
    /// Returns the set containing all possible values.
    pub fn full() -> Self {
        DataType::All
    }

    /// Returns the empty set.
    pub fn empty() -> Self {
        DataType::Some(vec![])
    }

    /// Remove all values in the set that do not satisfy the given constraint.
    pub fn constrain(&mut self, constraint: &constraints::constraint::Constraint) {
        todo!()
    }

    /// Returns whether the set contains the given value.
    pub fn contains(&self, value: &Arc<data_type::DataType>) -> bool {
        match self {
            DataType::All => true,
            DataType::Some(x) => x.iter().any(|x| x.matches(value)),
        }
    }

    /// Returns whether this is a superset of other, if further constraints
    /// imposed on any metavariables referred to by any data type patterns
    /// can't be futher constrained to change the outcome.
    pub fn superset_of(&self, other: &DataType) -> Option<bool> {
        match (self, other) {
            (DataType::All, _) => Some(true),
            (_, DataType::All) => Some(false),
            (DataType::Some(x), DataType::Some(y)) => {
                if y.is_empty() {
                    return Some(true);
                }
                if x.is_empty() {
                    return Some(false);
                }

                // All patterns in y must be covered by the union of patterns
                // in x. This is very difficult if x is not a single pattern!
                // For example, union(x) may "look like"
                // .-------.
                // |1 2 3 4|
                // |   .---'
                // |5 6|
                // '---'
                // as constructed from
                // .-------.
                // |1 2 3 4|
                // '-------'
                // and
                // .---.
                // |5 6|
                // '---'
                // for which it's difficult to prove that this covers
                // .---.
                // |1 2|
                // |   |
                // |5 6|
                // '---'
                // without having a way to construct all possible sets (rather
                // than just "rectangles"; where one dimension might be the
                // number of template parameters and the other might be the
                // variation) in one go. However, we can detect these cases by
                // also doing intersection checks, which are comparatively
                // easy; if any y intersects with more than one x the pattern
                // is too complicated and we return None. If solving the system
                // relies on this, this will simply yield a "failed to solve
                // system" diagnostic.

                let mut too_complex = false;
                for y in y {
                    let mut covered = false;
                    let mut num_intersections = 0;
                    for x in x {
                        if x.intersects_with(y) {
                            num_intersections += 1;
                            match x.covers(y) {
                                Some(true) => {
                                    covered = true;
                                    break;
                                }
                                Some(false) => {
                                    continue;
                                }
                                None => {
                                    return None;
                                }
                            }
                        }
                    }
                    if !covered {
                        return Some(false);
                    }
                    if num_intersections > 1 {
                        too_complex = true;
                    }
                }
                if too_complex {
                    None
                } else {
                    Some(true)
                }
            }
        }
    }

    /// Returns whether this set intersects with the other. Note that further
    /// constraints imposed on either set can only ever flip this outcome from
    /// true to false.
    pub fn intersects_with(&self, other: &DataType) -> bool {
        match (self, other) {
            (DataType::All, _) => !other.is_empty(),
            (_, DataType::All) => !other.is_empty(),
            (DataType::Some(x), DataType::Some(y)) => {
                for x in x {
                    for y in y {
                        if x.intersects_with(y) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    /// Returns whether this is the empty set.
    pub fn is_empty(&self) -> bool {
        match self {
            DataType::All => false,
            DataType::Some(x) => x.is_empty(),
        }
    }

    /// If this set contains exactly one value, return it.
    pub fn value(&self) -> diagnostic::Result<Option<Arc<data_type::DataType>>> {
        if let DataType::Some(patterns) = self {
            if patterns.len() == 1 {
                return patterns[0].make_concrete().transpose();
            }
        }
        Ok(None)
    }
}

/// A set of metavalues of any supported metatype.
#[derive(Clone, Debug, PartialEq)]
pub struct Set {
    /// The booleans contained in the set.
    booleans: Boolean,

    /// The integers contained in the set.
    integers: Integer,

    /// The data types contained in the set.
    data_types: DataType,
}

impl Set {
    /// Returns the set containing all possible values.
    pub fn full() -> Self {
        Self {
            booleans: Boolean::full(),
            integers: Integer::full(),
            data_types: DataType::full(),
        }
    }

    /// Returns the empty set.
    pub fn empty() -> Self {
        Self {
            booleans: Boolean::empty(),
            integers: Integer::empty(),
            data_types: DataType::empty(),
        }
    }

    /// Remove all values in the set that do not satisfy the given constraint.
    pub fn constrain(&mut self, constraint: &constraints::constraint::Constraint) {
        self.booleans.constrain(constraint);
        self.integers.constrain(constraint);
        self.data_types.constrain(constraint);
    }

    /// Returns whether the set contains the given value.
    pub fn contains(&self, value: &metavalues::value::Value) -> bool {
        match value {
            metavalues::value::Value::Boolean(b) => self.booleans.contains(*b),
            metavalues::value::Value::Integer(i) => self.integers.contains(*i),
            metavalues::value::Value::DataType(d) => self.data_types.contains(d),
        }
    }

    /// Returns whether this is a superset of other, if further constraints
    /// imposed on any metavariables referred to by any data type patterns
    /// can't be futher constrained to change the outcome.
    pub fn superset_of(&self, other: &Set) -> Option<bool> {
        self.data_types.superset_of(&other.data_types).map(|result| {
            result && self.booleans.superset_of(&other.booleans) && self.integers.superset_of(&other.integers)
        })
    }

    /// Returns whether this set intersects with the other. Note that further
    /// constraints imposed on either set can only ever flip this outcome from
    /// true to false.
    pub fn intersects_with(&self, other: &Set) -> bool {
        self.booleans.intersects_with(&other.booleans) || self.integers.intersects_with(&other.integers) || self.data_types.intersects_with(&other.data_types)
    }

    /// Returns whether this is the empty set.
    pub fn is_empty(&self) -> bool {
        self.booleans.is_empty() && self.integers.is_empty() && self.data_types.is_empty()
    }

    /// If this set contains exactly one value, return it.
    pub fn value(&self) -> diagnostic::Result<Option<metavalues::value::Value>> {
        match (self.booleans.is_empty(), self.integers.is_empty(), self.data_types.is_empty()) {
            (false, true, true) => Ok(self.booleans.value().map(|x| x.into())),
            (true, false, true) => Ok(self.integers.value().map(|x| x.into())),
            (true, true, false) => self.data_types.value().map(|x| x.map(|x| x.into())),
            _ => Ok(None),
        }
    }
}

impl From<Boolean> for Set {
    fn from(x: Boolean) -> Self {
        Self {
            booleans: x,
            integers: Integer::empty(),
            data_types: DataType::empty(),
        }
    }
}

impl From<Integer> for Set {
    fn from(x: Integer) -> Self {
        Self {
            booleans: Boolean::empty(),
            integers: x,
            data_types: DataType::empty(),
        }
    }
}

impl From<DataType> for Set {
    fn from(x: DataType) -> Self {
        Self {
            booleans: Boolean::empty(),
            integers: Integer::empty(),
            data_types: x,
        }
    }
}
