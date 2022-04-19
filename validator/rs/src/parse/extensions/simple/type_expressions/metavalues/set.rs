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

    /// The set is empty.
    None,
}

impl DataType {
    /// Returns the set containing all possible values.
    pub fn full() -> Self {
        DataType::All
    }

    /// Returns the empty set.
    pub fn empty() -> Self {
        DataType::None
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
            DataType::None => false,
        }
    }

    /// Returns whether this is the empty set.
    pub fn is_empty(&self) -> bool {
        matches!(self, DataType::None)
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
