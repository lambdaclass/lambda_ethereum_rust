use libmdbx::orm::{Decodable, Encodable};

pub struct BlockHeaderRLP(Vec<u8>);

impl Encodable for BlockHeaderRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for BlockHeaderRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(BlockHeaderRLP(b.to_vec()))
    }
}

// TODO: replace with actual types
pub type SyncAggregate = B256;
pub type ExecutionPayload = B256;
pub type BLSSignature = B256;
pub type Eth1Data = B256;

// The body of a block on the chain
// source: https://ethereum.org/en/developers/docs/consensus-mechanisms/pos/block-proposal/#how-is-a-block-created
// TODO: replace with actual types
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockBody {
    randao_reveal: BLSSignature,
    eth1_data: Eth1Data,
    graffiti: B256,
    proposer_slashings: Vec<B256>, // List[ProposerSlashing, MAX_PROPOSER_SLASHINGS]
    attester_slashings: Vec<B256>, // List[AttesterSlashing, MAX_ATTESTER_SLASHINGS]
    attestations: Vec<B256>,       // List[Attestation, MAX_ATTESTATIONS],
    deposits: Vec<B256>,           // List[Deposit, MAX_DEPOSITS],
    voluntary_exits: Vec<B256>,    // List[SignedVoluntaryExit, MAX_VOLUNTARY_EXITS],
    sync_aggregate: SyncAggregate,
    execution_payload: ExecutionPayload,
}

pub struct BlockBodyRLP(Vec<u8>);

impl Encodable for BlockBodyRLP {
    type Encoded = Vec<u8>;

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decodable for BlockBodyRLP {
    fn decode(b: &[u8]) -> anyhow::Result<Self> {
        Ok(BlockBodyRLP(b.to_vec()))
    }
}
