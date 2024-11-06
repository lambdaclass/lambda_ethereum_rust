use crate::{node::Node, node_hash::NodeHash, PathRLP, Trie, ValueRLP};

pub struct TrieIterator {
    trie: Trie,
    stack: Vec<NodeHash>,
}

impl TrieIterator {
    pub(crate) fn new(trie: Trie) -> Self {
        let stack = if let Some(root) = &trie.root {
            vec![root.clone()]
        } else {
            vec![]
        };
        Self { trie, stack }
    }
}

impl Iterator for TrieIterator {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stack.is_empty() {
            return None;
        };
        // Fetch the last node in the stack
        let next_node_hash = self.stack.pop()?;
        let next_node = self.trie.state.get_node(next_node_hash).ok()??;
        match &next_node {
            Node::Branch(branch_node) => {
                // Add all children to the stack (in reverse order so we process first child frist)
                for child in branch_node.choices.iter().rev() {
                    if child.is_valid() {
                        self.stack.push(child.clone())
                    }
                }
            }
            Node::Extension(extension_node) => {
                // Add child to the stack
                self.stack.push(extension_node.child.clone());
            }
            Node::Leaf(_) => {}
        }
        Some(next_node)
    }
}

impl TrieIterator {
    // TODO: construct path from nibbles
    pub fn content(self) -> impl Iterator<Item = (PathRLP, ValueRLP)> {
        self.filter_map(|n| match n {
            Node::Branch(branch_node) => {
                (!branch_node.value.is_empty()).then_some((vec![], branch_node.value))
            }
            Node::Extension(_) => None,
            Node::Leaf(leaf_node) => Some((vec![], leaf_node.value)),
        })
    }
}

pub fn print_trie(trie: &Trie) {
    let Some(root) = &trie.root else { return };
    print_node(trie, root.clone());
    print!("\n")
}

pub fn print_node(trie: &Trie, node_hash: NodeHash) {
    match trie.state.get_node(node_hash).unwrap().unwrap() {
        Node::Branch(n) => {
            print!("Branch{:?} [", n.value);
            for (i, child) in n.choices.iter().enumerate() {
                if child.is_valid() {
                    print!(" {i}: ");
                    print_node(trie, child.clone());
                }
            }
            print!(" ]")
        }
        Node::Extension(n) => {
            print!("Ext{:?} -> ", n.prefix.as_ref());
            print_node(trie, n.child);
        }
        Node::Leaf(n) => print!("Leaf{:?}{:?}", n.partial.as_ref(), n.value),
    }
}
