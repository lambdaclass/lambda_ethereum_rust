use libmdbx::orm::Encodable;

impl Encodable for LeafNode {
    type Encoded;

    fn encode(self) -> Self::Encoded {
        todo!()
    }
}