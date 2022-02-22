use crate::data_type;
use crate::path;
use crate::tree;
use std::collections::HashMap;
use std::rc::Rc;

/// Information about a YAML extension.
#[derive(Clone, Debug, PartialEq)]
pub struct YamlInfo {
    /// URI for the YAML file.
    pub uri: String,

    /// The path to the node that defined the anchor, if any.
    pub anchor_path: Option<path::PathBuf>,

    /// Parse result of this YAML file, if we resolved it.
    pub data: Option<YamlData>,
}

impl Default for YamlInfo {
    fn default() -> Self {
        Self {
            uri: "<unknown>".to_string(),
            anchor_path: None,
            data: None,
        }
    }
}

impl std::fmt::Display for YamlInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uri)
    }
}

/// Data for a resolved YAML file.
#[derive(Clone, Debug, PartialEq)]
pub struct YamlData {
    /// Reference to the parsed YAML data, if any.
    pub data: tree::NodeReference,

    /// Functions defined in this YAML file. Names are stored in lower case
    /// (Substrait's name resolution is case-insensitive).
    pub functions: HashMap<String, Rc<Function>>,

    /// Types defined in this YAML file. Names are stored in lower case
    /// (Substrait's name resolution is case-insensitive).
    pub types: HashMap<String, Rc<DataType>>,

    /// Type variations defined in this YAML file. Names are stored in lower
    /// case (Substrait's name resolution is case-insensitive).
    pub type_variations: HashMap<String, Rc<TypeVariation>>,
}

/// Extension information common to all extension types: URI, name, anchor
/// resolution information, and references to raw data.
#[derive(Clone, Debug, PartialEq)]
pub struct Common {
    /// The name of the type, type variation, or function.
    pub name: String,

    /// Information about the YAML that this extension is defined in, if any.
    pub yaml_info: Option<Rc<YamlInfo>>,

    /// The path to the node that defined the anchor for this extension, if
    /// any.
    pub anchor_path: Option<path::PathBuf>,
}

impl Default for Common {
    fn default() -> Self {
        Self {
            name: "<unknown>".to_string(),
            yaml_info: None,
            anchor_path: None,
        }
    }
}

impl std::fmt::Display for Common {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.yaml_info {
            Some(ref data) => write!(f, "{:?}", data.uri),
            None => write!(f, "<unknown>"),
        }?;
        write!(f, ".{}", self.name)
    }
}

/// Named/namespaced reference to a particular extension definition.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Reference<T> {
    /// Information common to all extension types.
    pub common: Common,

    /// Extension definition information, specific to this type of extension,
    /// if we managed to resolve the reference.
    pub definition: Option<Rc<T>>,
}

impl<T> std::fmt::Display for Reference<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.common.fmt(f)
    }
}

/// User-defined base data type.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DataType {
    /// The underlying structure of the type.
    pub structure: Vec<(String, data_type::Simple)>,
}

/// Type variation extension.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TypeVariation {
    /// Function behavior for this variation.
    pub behavior: FunctionBehavior,
}

/// Type variation function behavior.
#[derive(Clone, Debug, PartialEq)]
pub enum FunctionBehavior {
    Inherits,
    Separate,
}

impl Default for FunctionBehavior {
    fn default() -> Self {
        FunctionBehavior::Inherits
    }
}

/// Function extension.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Function {
    // TODO: need much more information here to do type checking.
}