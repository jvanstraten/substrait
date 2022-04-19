use crate::output::data_type;
use crate::output::data_type::ParameterInfo;
use crate::output::diagnostic;
use crate::util;
use crate::parse::extensions::simple::type_expressions::metavars;
use crate::parse::extensions::simple::type_expressions::metavalues;
use std::sync::Arc;

/// A pattern that matches some set of data types.
/// 
/// Types are printed/parsed in the following order:
/// 
///  - class;
///  - nullability;
///  - variation;
///  - parameter pack.
/// 
/// Intentionally convoluted example: `struct?x[?]<>` matches any variation of
/// an empty struct with nullability `x`.
/// 
/// When a data type pattern is successfully matched against a concrete type,
/// this may impose constraints on metavariables referenced in the pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    /// Type class (simple, compound, or user-defined).
    pub class: data_type::Class,

    /// Nullability. Must map to a boolean metavariable.
    ///  - generic -> printed/parsed as `class?generic`.
    ///  - anonymous -> printed/parsed as `class?_123`.
    ///  - resolved to true -> printed/parsed as `class?`.
    ///  - resolved to false -> printed/parsed as `class`.
    pub nullable: metavars::reference::Reference,

    /// Type variation, if specified. Note that data_type::Variation is itself
    /// an option:
    ///  - None -> variation is unspecified; this parameterized type matches
    ///    any variation. Printed/parsed as `class[?]`.
    ///  - Some(None) -> this parameterized type is the base variation of
    ///    class. Printed as `class`, parsed as `class` or `class[]`.
    ///  - Some(Some(variation)) -> this parameterized type is the specified
    ///    variation of class. Printed/parsed as `class[variation]`.
    pub variation: Option<data_type::Variation>,

    /// Parameters for parameterized types. Must be set to Some([]) for
    /// non-parameterizable types.
    ///  - None -> parameters are unspecified. Any number of parameters can be
    ///    matched, within the constraints of class. Printed/parsed as `class`,
    ///    even if class requires parameters.
    ///  - Some([]) -> parameters are specified to be an empty list.
    ///    Printed/parsed as `class<>`
    ///  - Some([a, b, c]) -> printed/parsed as `class<a, b, c>`.
    pub parameters: Option<Vec<Parameter>>,
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Class description.
        write!(f, "{}", self.class)?;

        // Nullable flag.
        match self.nullable.value().as_ref().and_then(metavalues::value::Value::as_boolean) {
            Some(true) => write!(f, "?")?,
            Some(false) => (),
            None => write!(f, "?{}", self.nullable)?,
        }

        // Variation.
        match &self.variation {
            Some(Some(variation)) => write!(f, "[{}]", variation)?,
            Some(None) => (),
            None => write!(f, "[?]")?,
        }

        // Parameter pack.
        if self.class.has_parameters() {
            if let Some(parameters) = &self.parameters {
                write!(f, "<")?;
                let mut first = true;
                for parameter in parameters.iter() {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{parameter}")?;
                }
                write!(f, ">")?;
            }
        }

        Ok(())
    }
}

impl Pattern {
    /// Bind all metavariable references in this pattern to the given context.
    pub fn bind(&mut self, context: &mut Context) {
        self.nullable.bind(context);
        if let Some(parameters) = &mut self.parameters {
            for parameter in parameters.iter_mut() {
                parameter.value.bind(context);
            }
        }
    }

    /// Add constraints to all referenced metavariables based on the pattern:
    ///  - the metavariable used to specify nullability must be a boolean;
    ///  - metavariables used in the parameter pack must satisfy the
    ///    constraints imposed by the class;
    ///  - if the parameter pack has the wrong number of parameters for the
    ///    class, Err is returned;
    ///  - if a parameter has a name and the class does not support this or
    ///    vice versa, Err is returned.
    pub fn apply_static_constraints(&self) -> diagnostic::Result<()> {
        todo!();
    }

    /// Returns whether the given concrete type matches this pattern. Parameter
    /// names are ignored in the comparison.
    pub fn matches(&self, concrete: &Arc<data_type::DataType>) -> bool {
        // Check class.
        if &self.class != concrete.class() {
            return false;
        }

        // Check nullability.
        if let Some(nullable) = self.nullable.value().as_ref().and_then(metavalues::value::Value::as_boolean) {
            if nullable != concrete.nullable() {
                return false;
            }
        }

        // Check variation.
        if let Some(variation) = &self.variation {
            if variation != concrete.variation() {
                return false;
            }
        }

        // Check parameter pack.
        if let Some(parameters) = &self.parameters {
            let concrete_parameters = concrete.parameters();
            if parameters.len() != concrete_parameters.len() {
                return false;
            }
            if parameters.iter().zip(concrete_parameters.iter()).any(|(x, y)| !x.matches(y)) {
                return false;
            }
        }
        
        return true;
    }

    /// Add constraints to all referenced parameters based on the given
    /// concrete type (effectively forcing the values of the metavariables)
    /// and copy the variation from the pattern.
    pub fn apply_match_constraints(&mut self, concrete: &Arc<data_type::DataType>) -> diagnostic::Result<()> {
        todo!();
    }

    /// Checks whether this pattern covers another, i.e. all types that
    /// match other also match this.
    pub fn covers(&self, other: &Pattern) -> bool {
        todo!()
    }

    /// Returns the concrete type associated with this pattern, if it is a
    /// concrete type. An error is contained in the option if this is a
    /// concrete type but the type could not be constructed because it is
    /// invalid.
    pub fn make_concrete(&self) -> Option<diagnostic::Result<Arc<data_type::DataType>>> {
        todo!();
    }
}

/// A parameter within a data type parameter pack.
/// 
/// Printed/parsed as:
/// 
///  - `name: value` for named parameters;
///  - `value` for non-named parameters.
#[derive(Clone, Debug)]
pub struct Parameter {
    /// Name of this parameter, if applicable (currently used only for
    /// NSTRUCT).
    pub name: Option<String>,

    /// The metavariable representing the value of this parameter.
    pub value: metavars::reference::Reference,
}

impl PartialEq for Parameter {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl std::fmt::Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{}: ", util::string::as_ident_or_string(name))?;
        }
        write!(f, "{}", self.value)
    }
}

impl Parameter {
    /// Returns whether the given parameter value matches one of the remaining
    /// possible values for value. The parameter name is not checked.
    pub fn matches(&self, parameter: &data_type::Parameter) -> bool {
        match parameter {
            data_type::Parameter::Type(_) => todo!(),
            data_type::Parameter::NamedType(_, _) => todo!(),
            data_type::Parameter::Unsigned(_) => todo!(),
        }
    }
}
