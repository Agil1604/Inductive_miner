use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::{LeafType, Node, OperatorType, ProcessTree};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct JsonTree<'a> {
    root: JsonNode<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum JsonNode<'a> {
    Activity {
        name: &'a str,
    },
    Tau,
    Operator {
        operator: &'static str,
        children: Vec<JsonNode<'a>>,
    },
}

impl<'a> From<&'a Node> for JsonNode<'a> {
    fn from(node: &'a Node) -> Self {
        match node {
            Node::Leaf(leaf) => match &leaf.activity_label {
                LeafType::Activity(a) => Self::Activity { name: &a.0 },
                LeafType::Tau => Self::Tau,
            },
            Node::Operator(op) => Self::Operator {
                operator: match op.operator_type {
                    OperatorType::Sequence => "sequence",
                    OperatorType::Xor => "xor",
                    OperatorType::Concurrent => "concurrent",
                    OperatorType::Loop => "loop",
                    OperatorType::Interleaved => "interleaved",
                    OperatorType::InclusiveChoice => "inclusive_choice",
                },
                children: op.children.iter().map(Self::from).collect(),
            },
        }
    }
}

impl<'a> From<&'a ProcessTree> for JsonTree<'a> {
    fn from(tree: &'a ProcessTree) -> Self {
        Self {
            root: tree.root().into(),
        }
    }
}

/// Writes pretty JSON to a file, creating it or replacing its contents.
/// Activity names and child order are preserved; tau is distinct from a name.
///
/// # Errors
/// Returns an I/O error if opening, writing, or flushing the file fails.
///
/// ```no_run
/// use std::path::Path;
/// use inductive_miner::{Node, ProcessTree, io::write_json};
/// let tree = ProcessTree::new(Node::new_leaf(None))?;
/// write_json(Path::new("tree.json"), &tree)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn write_json(path: &Path, tree: &ProcessTree) -> std::io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    write_json_to_writer(&mut writer, tree)?;
    writer.flush()
}

/// Writes pretty JSON to a byte writer without flushing it.
///
/// # Errors
/// Returns an I/O error if the destination cannot accept the JSON bytes.
pub fn write_json_to_writer<W: Write>(writer: W, tree: &ProcessTree) -> std::io::Result<()> {
    serde_json::to_writer_pretty(writer, &JsonTree::from(tree)).map_err(std::io::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Activity;
    use serde_json::json;

    #[test]
    fn nested_tree_preserves_names_tau_and_order() {
        let tree = ProcessTree::new(Node::new_operator_with_children(
            OperatorType::Sequence,
            vec![
                Node::new_leaf(Some(Activity::from("A\"\n\\雪"))),
                Node::new_operator_with_children(
                    OperatorType::Loop,
                    vec![
                        Node::new_leaf(None),
                        Node::new_leaf(Some(Activity::from("tau"))),
                    ],
                ),
            ],
        ))
        .unwrap();
        let expected = json!({"root":{"type":"operator","operator":"sequence","children":[
            {"type":"activity","name":"A\"\n\\雪"},
            {"type":"operator","operator":"loop","children":[{"type":"tau"},{"type":"activity","name":"tau"}]}
        ]}});
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&tree.to_json().unwrap()).unwrap(),
            expected
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&tree.to_json_pretty().unwrap()).unwrap(),
            expected
        );
        let mut output = Vec::new();
        write_json_to_writer(&mut output, &tree).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output).unwrap(),
            expected
        );
    }

    #[test]
    fn all_operator_names_are_exported() {
        for (op, name) in [
            (OperatorType::Sequence, "sequence"),
            (OperatorType::Xor, "xor"),
            (OperatorType::Concurrent, "concurrent"),
            (OperatorType::Loop, "loop"),
            (OperatorType::Interleaved, "interleaved"),
            (OperatorType::InclusiveChoice, "inclusive_choice"),
        ] {
            let tree = ProcessTree::new(Node::new_operator_with_children(
                op,
                vec![Node::new_leaf(None), Node::new_leaf(None)],
            ))
            .unwrap();
            let value: serde_json::Value = serde_json::from_str(&tree.to_json().unwrap()).unwrap();
            assert_eq!(value["root"]["operator"], name);
        }
    }

    #[test]
    fn writer_errors_are_propagated() {
        struct Failing;
        impl Write for Failing {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let tree = ProcessTree::new(Node::new_leaf(None)).unwrap();
        assert_eq!(
            write_json_to_writer(Failing, &tree).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
}
