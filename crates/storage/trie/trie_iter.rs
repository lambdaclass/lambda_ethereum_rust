use crate::{node::Node, node_hash::NodeHash, Trie};

pub struct TrieIterator<'a> {
    trie: &'a Trie,
    stack: Vec<NodeHash>,
}

impl<'a> TrieIterator<'a> {
    pub(crate) fn new(trie: &'a Trie) -> Self {
        let stack = if let Some(root) = &trie.root {
            vec![root.clone()]
        } else {
            vec![]
        };
        Self { trie, stack }
    }
}

impl<'a> Iterator for TrieIterator<'a> {
    // First iteration => return node, then check if returning key-value pairs is better
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stack.is_empty() {
            return None;
        };
        // Fetch the last node in the stack
        let next_node_hash = self.stack.pop().unwrap();
        let next_node = self.trie.state.get_node(next_node_hash).unwrap().unwrap();
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
        return Some(next_node);
    }
}
