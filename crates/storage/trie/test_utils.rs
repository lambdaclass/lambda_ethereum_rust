use super::Trie;

pub fn start_trie(trie_dir: &str) -> Trie {
    remove_trie(trie_dir); // In case a trie db was left from a previous test execution
    Trie::new(trie_dir).expect("Failed to create Trie")
}

pub fn remove_trie(trie_dir: &str) {
    if std::path::Path::new(trie_dir).exists() {
        std::fs::remove_dir_all(trie_dir).expect("Failed to clean test db dir");
    }
}

#[macro_export]
macro_rules! pmt_node {
    (
        @( $trie:expr )
        branch { $( $choice:expr => $child_type:ident { $( $child_tokens:tt )* } ),+ $(,)? }
        $( offset $offset:expr )?
    ) => {
        $crate::trie::node::BranchNode::new({
            #[allow(unused_variables)]
            let offset = true $( ^ $offset )?;
            let mut choices = [$crate::trie::node_ref::NodeRef::default(); 16];
            $(
                let child_node = pmt_node! { @($trie)
                    $child_type { $( $child_tokens )* }
                    offset offset
                }.into();
                let child_node = $trie.db.insert_node(child_node).unwrap();
                choices[$choice as usize] = child_node;
            )*
            choices
        })
    };
    (
        @( $trie:expr )
        branch { $( $choice:expr => $child_type:ident { $( $child_tokens:tt )* } ),+ $(,)? }
        with_leaf { $path:expr => $value:expr }
        $( offset $offset:expr )?
    ) => {{
        let mut branch_node = $crate::trie::node::BranchNode::new({
            #[allow(unused_variables)]
            let offset = true $( ^ $offset )?;
            let mut choices = [$crate::trie::node_ref::NodeRef::default(); 16];
            $(
                choices[$choice as usize] = $trie.db.insert_node(
                    pmt_node! { @($trie)
                        $child_type { $( $child_tokens )* }
                        offset offset
                    }.into()
                ).unwrap();
            )*
            choices
        });
        $trie.db.insert_value($path, $value).unwrap();
        branch_node.update_path($path);
        branch_node
    }};

    (
        @( $trie:expr )
        extension { $prefix:expr , $child_type:ident { $( $child_tokens:tt )* } }
        $( offset $offset:expr )?
    ) => {{
        #[allow(unused_variables)]
        let offset = false $( ^ $offset )?;
        let prefix = $crate::trie::nibble::NibbleVec::from_nibbles(
            $prefix
                .into_iter()
                .map(|x: u8| $crate::trie::nibble::Nibble::try_from(x).unwrap()),
            offset
        );

        let offset = offset  ^ (prefix.len() % 2 != 0);
        $crate::trie::node::ExtensionNode::new(
            prefix,
            {
                let child_node = pmt_node! { @($trie)
                    $child_type { $( $child_tokens )* }
                    offset offset
                }.into();
                $trie.db.insert_node(child_node).unwrap()
            }
        )
    }};

    (
        @( $trie:expr)
        leaf { $path:expr => $value:expr }
        $( offset $offset:expr )?
    ) => {
        {
            $trie.db.insert_value($path.clone(), $value).unwrap();
            $crate::trie::node::LeafNode::new($path)
        }
    };
}

#[macro_export]
macro_rules! pmt_path {
    ( $path:literal ) => {{
        assert!($path.len() % 2 == 1);
        $path
            .as_bytes()
            .chunks(2)
            .map(|bytes| u8::from_str_radix(std::str::from_utf8(bytes).unwrap(), 16).unwrap())
            .collect::<Vec<u8>>()
    }};
}
