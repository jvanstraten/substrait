use crate::comment;
use crate::context;
use crate::data_type;
use crate::diagnostic;
use crate::extension;
use crate::path;
use crate::primitives;
use crate::proto::meta::*;
use crate::tree;
use std::collections::VecDeque;
use std::rc::Rc;

/// Convenience/shorthand macro for pushing diagnostic messages to a node.
macro_rules! diagnostic {
    ($context:expr, $level:ident, $cause:ident, $($fmts:expr),*) => {
        diagnostic!($context, $level, diagnostic::Cause::$cause(format!($($fmts),*)))
    };
    ($context:expr, $level:ident, $cause:expr) => {
        tree::push_diagnostic($context, diagnostic::Level::$level, $cause)
    };
}

/// Pushes a diagnostic message to the node information list.
pub fn push_diagnostic(
    context: &mut context::Context,
    level: diagnostic::Level,
    cause: diagnostic::Cause,
) {
    context
        .output
        .data
        .push(NodeData::Diagnostic(diagnostic::Diagnostic {
            cause,
            level,
            path: context.breadcrumb.path.to_path_buf(),
        }))
}

/// Convenience/shorthand macro for pushing comments to a node.
#[allow(unused_macros)]
macro_rules! comment {
    ($context:expr, $($fmts:expr),*) => {
        tree::push_comment($context, format!($($fmts),*), None)
    };
}

/// Convenience/shorthand macro for pushing comments to a node.
#[allow(unused_macros)]
macro_rules! link {
    ($context:expr, $link:expr, $($fmts:expr),*) => {
        tree::push_comment($context, format!($($fmts),*), Some($link))
    };
}

/// Pushes a comment to the node information list.
#[allow(unused_macros)]
pub fn push_comment<S: AsRef<str>>(
    context: &mut context::Context,
    text: S,
    path: Option<path::PathBuf>,
) {
    let text = text.as_ref().to_string();
    let comment = comment::Comment::new();
    let comment = if let Some(path) = path {
        comment.with_link_to_path(text, path)
    } else {
        comment.with_plain(text)
    };
    context.output.data.push(NodeData::Comment(comment))
}

/// Convenience/shorthand macro for pushing type information to a node. Note
/// that this macro isn't shorter than just using push_data_type() directly; it
/// exists for symmetry.
#[allow(unused_macros)]
macro_rules! data_type {
    ($context:expr, $typ:expr) => {
        tree::push_data_type($context, $typ)
    };
}

/// Pushes a data type to the node information list, and saves it in the
/// current context.
pub fn push_data_type(context: &mut context::Context, data_type: data_type::DataType) {
    context
        .output
        .data
        .push(NodeData::DataType(data_type.clone()));
    context.output.data_type = Some(data_type);
}

/// Convenience/shorthand macro for parsing optional protobuf fields.
#[allow(unused_macros)]
macro_rules! proto_field {
    ($input:expr, $context:expr, $field:ident) => {
        proto_field!($input, $context, $field, |_, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr) => {
        proto_field!($input, $context, $field, $parser, |_, _, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr, $validator:expr) => {
        tree::push_proto_field(
            $input,
            $context,
            &$input.$field.as_ref(),
            stringify!($field),
            false,
            $parser,
            $validator,
        )
    };
}

#[allow(unused_macros)]
macro_rules! proto_boxed_field {
    ($input:expr, $context:expr, $field:ident) => {
        proto_boxed_field!($input, $context, $field, |_, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr) => {
        proto_boxed_field!($input, $context, $field, $parser, |_, _, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr, $validator:expr) => {
        tree::push_proto_field(
            $input,
            $context,
            &$input.$field,
            stringify!($field),
            false,
            $parser,
            $validator,
        )
    };
}

/// Parse and push a protobuf optional field.
pub fn push_proto_field<TP, TF, TR, FP, FV>(
    input: &TP,
    context: &mut context::Context,
    field: &Option<impl std::ops::Deref<Target = TF>>,
    field_name: &'static str,
    unknown_subtree: bool,
    parser: FP,
    validator: FV,
) -> (Option<Rc<Node>>, Option<TR>)
where
    TF: ProtoDatum,
    FP: Fn(&TF, &mut context::Context) -> diagnostic::Result<TR>,
    FV: Fn(&TP, &mut context::Context, &Node) -> diagnostic::Result<()>,
{
    if !context
        .breadcrumb
        .fields_parsed
        .insert(field_name.to_string())
    {
        panic!("field {} was parsed multiple times", field_name);
    }

    if let Some(field_input) = field {
        let field_input = field_input.deref();

        // Create the node for the child message.
        let mut field_output = field_input.proto_data_to_node();

        // Create the path element for referring to the child node.
        let path_element = if let Some(variant) = field_input.proto_data_variant() {
            path::PathElement::Variant(field_name.to_string(), variant.to_string())
        } else {
            path::PathElement::Field(field_name.to_string())
        };

        // Create the context for the child message.
        let mut field_context = context::Context {
            output: &mut field_output,
            state: context.state,
            breadcrumb: &mut context.breadcrumb.next(path_element.clone()),
            config: context.config,
        };

        // Call the provided parser function.
        let result = parser(field_input, &mut field_context)
            .map_err(|cause| {
                diagnostic!(&mut field_context, Error, cause);
            })
            .ok();

        // Handle any fields not handled by the provided parse function.
        handle_unknown_fields(field_input, &mut field_context, unknown_subtree);

        // Push and return the completed node.
        let field_output = Rc::new(field_output);
        context.output.data.push(NodeData::Child(Child {
            path_element,
            node: field_output.clone(),
            recognized: !unknown_subtree,
        }));

        // Run the validator.
        if let Err(cause) = validator(input, context, &field_output) {
            diagnostic!(context, Error, cause);
        }

        (Some(field_output), result)
    } else {
        (None, None)
    }
}

/// Convenience/shorthand macro for parsing required protobuf fields.
#[allow(unused_macros)]
macro_rules! proto_required_field {
    ($input:expr, $context:expr, $field:ident) => {
        proto_required_field!($input, $context, $field, |_, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr) => {
        proto_required_field!($input, $context, $field, $parser, |_, _, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr, $validator:expr) => {
        tree::push_proto_required_field(
            $input,
            $context,
            &$input.$field.as_ref(),
            stringify!($field),
            false,
            $parser,
            $validator,
        )
    };
}

#[allow(unused_macros)]
macro_rules! proto_boxed_required_field {
    ($input:expr, $context:expr, $field:ident) => {
        proto_boxed_required_field!($input, $context, $field, |_, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr) => {
        proto_boxed_required_field!($input, $context, $field, $parser, |_, _, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr, $validator:expr) => {
        tree::push_proto_required_field(
            $input,
            $context,
            &$input.$field,
            stringify!($field),
            false,
            $parser,
            $validator,
        )
    };
}

#[allow(unused_macros)]
macro_rules! proto_primitive_field {
    ($input:expr, $context:expr, $field:ident) => {
        proto_primitive_field!($input, $context, $field, |x, _| Ok(x.to_owned()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr) => {
        proto_primitive_field!($input, $context, $field, $parser, |_, _, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr, $validator:expr) => {
        tree::push_proto_required_field(
            $input,
            $context,
            &Some(&$input.$field),
            stringify!($field),
            false,
            $parser,
            $validator,
        )
    };
}

/// Parse and push a required field of some message type. If the field is
/// not populated, a MissingField diagnostic is pushed automatically, and
/// an empty node is returned as an error recovery placeholder.
pub fn push_proto_required_field<TP, TF, TR, FP, FV>(
    input: &TP,
    context: &mut context::Context,
    field: &Option<impl std::ops::Deref<Target = TF>>,
    field_name: &'static str,
    unknown_subtree: bool,
    parser: FP,
    validator: FV,
) -> (Rc<Node>, Option<TR>)
where
    TF: ProtoDatum,
    FP: Fn(&TF, &mut context::Context) -> diagnostic::Result<TR>,
    FV: Fn(&TP, &mut context::Context, &Node) -> diagnostic::Result<()>,
{
    if let (Some(node), result) = push_proto_field(
        input,
        context,
        field,
        field_name,
        unknown_subtree,
        parser,
        validator,
    ) {
        (node, result)
    } else {
        diagnostic!(context, Error, MissingField, "{}", field_name);
        (Rc::new(TF::proto_type_to_node()), None)
    }
}

/// Convenience/shorthand macro for parsing repeated protobuf fields.
macro_rules! proto_repeated_field {
    ($input:expr, $context:expr, $field:ident) => {
        proto_repeated_field!($input, $context, $field, |_, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr) => {
        proto_repeated_field!($input, $context, $field, $parser, |_, _, _, _| Ok(()))
    };
    ($input:expr, $context:expr, $field:ident, $parser:expr, $validator:expr) => {
        tree::push_proto_repeated_field(
            $input,
            $context,
            &$input.$field,
            stringify!($field),
            false,
            $parser,
            $validator,
        )
    };
}

/// Parse and push a repeated field of some message type. If specified, the
/// given validator function will be called in the current context
/// immediately after each repetition of the field is handled, allowing
/// field-specific validation to be done.
pub fn push_proto_repeated_field<TP, TF, TR, FP, FV>(
    input: &TP,
    context: &mut context::Context,
    field: &[TF],
    field_name: &'static str,
    unknown_subtree: bool,
    parser: FP,
    validator: FV,
) -> (Vec<Rc<Node>>, Vec<Option<TR>>)
where
    TF: ProtoDatum,
    FP: Fn(&TF, &mut context::Context) -> diagnostic::Result<TR>,
    FV: Fn(&TP, &mut context::Context, &Node, usize) -> diagnostic::Result<()>,
{
    if !context
        .breadcrumb
        .fields_parsed
        .insert(field_name.to_string())
    {
        panic!("field {} was parsed multiple times", field_name);
    }

    field
        .iter()
        .enumerate()
        .map(|(index, field_input)| {
            // Create the node for the child message.
            let mut field_output = field_input.proto_data_to_node();

            // Create the path element for referring to the child node.
            let path_element = path::PathElement::Repeated(field_name.to_string(), index);

            // Create the context for the child message.
            let mut field_context = context::Context {
                output: &mut field_output,
                state: context.state,
                breadcrumb: &mut context.breadcrumb.next(path_element.clone()),
                config: context.config,
            };

            // Call the provided parser function.
            let result = parser(field_input, &mut field_context)
                .map_err(|cause| {
                    diagnostic!(&mut field_context, Error, cause);
                })
                .ok();

            // Handle any fields not handled by the provided parse function.
            handle_unknown_fields(field_input, &mut field_context, unknown_subtree);

            // Push the completed node.
            let field_output = Rc::new(field_output);
            context.output.data.push(NodeData::Child(Child {
                path_element,
                node: field_output.clone(),
                recognized: !unknown_subtree,
            }));

            // Run the validator.
            if let Err(cause) = validator(input, context, &field_output, index) {
                diagnostic!(context, Error, cause);
            }

            (field_output, result)
        })
        .unzip()
}

/// Handle all fields that haven't already been handled. If unknown_subtree
/// is false, this also generates a diagnostic message if there were
/// populated/non-default unhandled fields.
fn handle_unknown_fields<T: ProtoDatum>(
    input: &T,
    context: &mut context::Context,
    unknown_subtree: bool,
) {
    if input.proto_parse_unknown(context) && !unknown_subtree {
        let mut fields = vec![];
        for data in context.output.data.iter() {
            if let NodeData::Child(child) = data {
                if !child.recognized {
                    fields.push(child.path_element.to_string());
                }
            }
        }
        if !fields.is_empty() {
            let fields: String =
                itertools::Itertools::intersperse(fields.into_iter(), ", ".to_string()).collect();
            diagnostic!(context, Warning, UnknownField, "{}", fields);
        }
    }
}

/// Node for a semi-structured documentation-like tree representation of a
/// parsed Substrait plan. The intention is for this to be serialized into
/// some human-readable format.
///
/// Note: although it should be possible to reconstruct the entire plan from
/// the information contained in the tree, the tree is only intended to be
/// converted to structured human-readable documentation for the plan. It is
/// expressly NOT intended to be read as a form of AST by a downstream
/// process, and therefore isn't nearly as strictly-typed as you would
/// otherwise want it to be. Protobuf itself is already a reasonable format
/// for this!
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The type of node.
    pub node_type: NodeType,

    /// The type of data returned by this node, if any. Depending on the
    /// message and context, this may represent a table schema or scalar
    /// data.
    pub data_type: Option<data_type::DataType>,

    /// The information gathered about the message.
    ///
    /// This normally includes all child nodes for this message, possibly
    /// interspersed with diagnostics, type information, and unstructured
    /// comment nodes to provide context, all ordered in a reasonable way.
    /// Note however that this information is intended to be understood by
    /// a human, not by the validator itself (aside from serialization to a
    /// human-readable notation).
    pub data: Vec<NodeData>,
}

impl From<NodeType> for Node {
    fn from(node_type: NodeType) -> Self {
        Node {
            node_type,
            data_type: None,
            data: vec![],
        }
    }
}

impl Node {
    /// Parses/validates the given binary serialization of a protobuffer using
    /// the given (root) parser/validator.
    pub fn parse_proto<T, F, B>(
        buffer: B,
        root_name: &'static str,
        root_parser: F,
        state: &mut context::State,
        config: &context::Config,
    ) -> Self
    where
        T: prost::Message + ProtoDatum + Default,
        F: FnOnce(&T, &mut context::Context) -> diagnostic::Result<()>,
        B: prost::bytes::Buf,
    {
        match T::decode(buffer) {
            Err(err) => {
                // Create a minimal root node with just the prot

                let mut output = T::proto_type_to_node();
                output
                    .data
                    .push(NodeData::Diagnostic(diagnostic::Diagnostic {
                        cause: err.into(),
                        level: diagnostic::Level::Error,
                        path: path::PathBuf {
                            root: root_name,
                            elements: vec![],
                        },
                    }));
                output
            }
            Ok(input) => {
                // Create the root node.
                let mut output = input.proto_data_to_node();

                // Create the root context.
                let mut context = context::Context {
                    output: &mut output,
                    state,
                    breadcrumb: &mut context::Breadcrumb::new(root_name),
                    config,
                };

                // Call the provided parser function.
                if let Err(cause) = root_parser(&input, &mut context) {
                    diagnostic!(&mut context, Error, cause);
                }

                // Handle any fields not handled by the provided parse function.
                handle_unknown_fields(&input, &mut context, false);

                output
            }
        }
    }

    /// Returns an iterator that iterates over all nodes depth-first.
    pub fn iter_flattened_nodes(&self) -> FlattenedNodeIter {
        FlattenedNodeIter {
            remaining: VecDeque::from(vec![self]),
        }
    }

    /// Returns an iterator that iterates over all NodeData objects in the
    /// order in which they were defined.
    pub fn iter_flattened_node_data(&self) -> FlattenedNodeDataIter {
        FlattenedNodeDataIter {
            remaining: self.data.iter().rev().collect(),
        }
    }

    /// Iterates over all diagnostics in the tree.
    pub fn iter_diagnostics(&self) -> impl Iterator<Item = &diagnostic::Diagnostic> + '_ {
        self.iter_flattened_node_data().filter_map(|x| match x {
            NodeData::Diagnostic(d) => Some(d),
            _ => None,
        })
    }
}

/// The original data type that the node represents, to (in theory) allow the
/// original structure of the plan to be recovered from the documentation tree.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeType {
    /// The associated node represents a protobuf message of the given type
    /// (full protobuf path). The contents of the message are described using
    /// Field, RepeatedField, and OneOfField.
    ProtoMessage(&'static str),

    /// The associated node represents a protobuf primitive value of the given
    /// type and with the given data.
    ProtoPrimitive(&'static str, primitives::PrimitiveData),

    /// The associated node represents an unpopulated oneof field. This should
    /// never appear in the final tree, but is used when a Rust enum
    /// representation of a oneof field is converted to a node without data.
    ProtoMissingOneOf,

    /// Used for anchor/reference-based references to other nodes.
    Reference(u64, NodeReference),

    /// Used for resolved YAML URIs, in order to include the parse result and
    /// documentation for the referenced YAML (if available), in addition to
    /// the URI itself.
    YamlData(Rc<extension::YamlInfo>),

    /// The associated node represents a YAML map. The contents of the map are
    /// described using Field and UnknownField.
    YamlMap,

    /// The associated node represents a YAML array. The contents of the array
    /// are described using ArrayElement datums.
    YamlArray,

    /// The associated node represents a YAML primitive.
    YamlPrimitive(primitives::PrimitiveData),
}

/// Information nodes for a parsed protobuf message.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeData {
    /// A reference to a child node in the tree.
    Child(Child),

    /// Indicates that parsing/validating this message resulted in some
    /// diagnostic message being emitted.
    Diagnostic(diagnostic::Diagnostic),

    /// Provides (intermediate) type information for this node. Depending on
    /// the message, this may be a struct or named struct representing a
    /// schema, or it may represent the type of some scalar expression.
    /// Multiple TypeInfo nodes may be present, in particular for relations
    /// that perform multiple operations in one go (for example read, project,
    /// emit). The TypeInfo and operation description *Field nodes are then
    /// ordered by data flow. In particular, the last TypeInfo node always
    /// represents the type of the final result of a node.
    DataType(data_type::DataType),

    /// Used for adding unstructured additional information to a message,
    /// wherever this may aid human understanding of a message.
    Comment(comment::Comment),
}

/// Reference to a child node in the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Child {
    /// Path element identifying the relation of this child node to its parent.
    pub path_element: path::PathElement,

    /// The child node.
    pub node: Rc<Node>,

    /// Whether the validator recognized/expected the field or element that
    /// this child represents. Fields/elements may be unrecognized simply
    /// because validation is not implemented for them yet. In any case, this
    /// flag indicates that the subtree represented by this node could not be
    /// validated.
    pub recognized: bool,
}

/// A reference to a node elsewhere in the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeReference {
    /// Absolute path to the node.
    pub path: path::PathBuf,

    /// Link to the node.
    pub node: Rc<Node>,
}

pub struct FlattenedNodeIter<'a> {
    remaining: VecDeque<&'a Node>,
}

impl<'a> Iterator for FlattenedNodeIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        let maybe_node = self.remaining.pop_back();
        if let Some(node) = maybe_node {
            self.remaining
                .extend(node.data.iter().rev().filter_map(|x| -> Option<&Node> {
                    if let NodeData::Child(child) = x {
                        Some(&child.node)
                    } else {
                        None
                    }
                }));
        }
        maybe_node
    }
}

pub struct FlattenedNodeDataIter<'a> {
    remaining: VecDeque<&'a NodeData>,
}

impl<'a> Iterator for FlattenedNodeDataIter<'a> {
    type Item = &'a NodeData;

    fn next(&mut self) -> Option<Self::Item> {
        let maybe_node_data = self.remaining.pop_back();
        if let Some(NodeData::Child(child)) = maybe_node_data {
            self.remaining.extend(child.node.data.iter().rev())
        }
        maybe_node_data
    }
}