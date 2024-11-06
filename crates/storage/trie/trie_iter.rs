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
    let stack = if let Some(root) = &trie.root {
        vec![root.clone()]
    } else {
        vec![]
    };
    print_trie_inner(stack, trie);
}

pub fn print_trie_inner(mut stack: Vec<NodeHash>, trie: &Trie) {
    if stack.is_empty() {
        return;
    };
    // Fetch the last node in the stack
    let next_node_hash = stack.pop().unwrap();
    let next_node = trie.state.get_node(next_node_hash).ok().unwrap().unwrap();
    match &next_node {
        Node::Branch(branch_node) => {
            // Add all children to the stack (in reverse order so we process first child frist)
            print!("BranchNode {{ Children: [");
            for (i, child) in branch_node.choices.iter().enumerate().rev() {
                print!("{i}: {:?}", child.as_ref());
                if child.is_valid() {
                    stack.push(child.clone())
                }
            }
            print!("] Value: {:?} }}\n", branch_node.value);
        }
        Node::Extension(extension_node) => {
            // Add child to the stack
            println!(
                "ExtensionNode {{ Prefix: {:?} Child: {:?}}}",
                extension_node.prefix,
                extension_node.child.as_ref()
            );
            stack.push(extension_node.child.clone());
        }
        Node::Leaf(leaf) => {
            println!(
                "LeafNode {{ Partial: {:?} Value: {:?}}}",
                leaf.partial.as_ref(),
                leaf.value
            );
        }
    }
}
