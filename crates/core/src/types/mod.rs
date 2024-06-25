use crate::{rlp::encode::RLPEncode, Address, H256, U256};
use bytes::Bytes;

pub type BlockNumber = u64;
pub type Bloom = [u8; 256];

/// Header part of a block on the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    parent_hash: H256,
    ommers_hash: H256,
    coinbase: Address,
    state_root: H256,
    transactions_root: H256,
    receipt_root: H256,
    logs_bloom: Bloom,
    difficulty: U256,
    number: BlockNumber,
    gas_limit: u64,
    gas_used: u64,
    timestamp: u64,
    extra_data: Bytes,
    prev_randao: H256,
    nonce: u64,
    base_fee_per_gas: u64,
    withdrawals_root: H256,
    blob_gas_used: u64,
    excess_blob_gas: u64,
    parent_beacon_block_root: H256,
}

impl RLPEncode for BlockHeader {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.parent_hash.encode(buf);
        self.ommers_hash.encode(buf);
        self.coinbase.encode(buf);
        self.state_root.encode(buf);
        self.transactions_root.encode(buf);
        self.receipt_root.encode(buf);
        self.logs_bloom.encode(buf);

        // TODO: move to rlp::encode
        let mut tmp_buf = vec![];
        self.difficulty.to_big_endian(&mut tmp_buf);
        tmp_buf.encode(buf);

        self.number.encode(buf);
        self.gas_limit.encode(buf);
        self.gas_used.encode(buf);
        self.timestamp.encode(buf);
        self.extra_data.encode(buf);
        self.prev_randao.encode(buf);
        self.nonce.encode(buf);
        self.base_fee_per_gas.encode(buf);
        self.withdrawals_root.encode(buf);
        self.blob_gas_used.encode(buf);
        self.excess_blob_gas.encode(buf);
        self.parent_beacon_block_root.encode(buf);
    }
}

// TODO: replace with actual types
pub type SyncAggregate = H256;
pub type ExecutionPayload = H256;
pub type BLSSignature = H256;
pub type Eth1Data = H256;

// The body of a block on the chain
// source: https://ethereum.org/en/developers/docs/consensus-mechanisms/pos/block-proposal/#how-is-a-block-created
// TODO: replace with actual types
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockBody {
    randao_reveal: BLSSignature,
    eth1_data: Eth1Data,
    graffiti: H256,
    proposer_slashings: Vec<H256>, // List[ProposerSlashing, MAX_PROPOSER_SLASHINGS]
    attester_slashings: Vec<H256>, // List[AttesterSlashing, MAX_ATTESTER_SLASHINGS]
    attestations: Vec<H256>,       // List[Attestation, MAX_ATTESTATIONS],
    deposits: Vec<H256>,           // List[Deposit, MAX_DEPOSITS],
    voluntary_exits: Vec<H256>,    // List[SignedVoluntaryExit, MAX_VOLUNTARY_EXITS],
    sync_aggregate: SyncAggregate,
    execution_payload: ExecutionPayload,
}

impl RLPEncode for BlockBody {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.randao_reveal.encode(buf);
        self.eth1_data.encode(buf);
        self.graffiti.encode(buf);
        self.proposer_slashings.encode(buf);
        self.attester_slashings.encode(buf);
        self.attestations.encode(buf);
        self.deposits.encode(buf);
        self.voluntary_exits.encode(buf);
        self.sync_aggregate.encode(buf);
        self.execution_payload.encode(buf);
    }
}
