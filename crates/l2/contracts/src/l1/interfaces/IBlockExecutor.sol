// SPDX-License-Identifier: MIT
pragma solidity 0.8.27;

/// @title Interface for the BlockExecutor contract.
/// @author LambdaClass
/// @notice An Inbox contract is a contract that allows L1<->L2 communication
/// from L1. It both sends messages from L1 to L2 and receives messages from L2.
interface IBlockExecutor {
    /// @notice A block has been committed.
    /// @dev Event emitted when a block is committed.
    event BlockCommitted(bytes32 indexed previousBlockCommitment, bytes32 indexed currentBlockCommitment);

    /// @notice A block has been verified.
    /// @dev Event emitted when a block is verified.
    event BlockVerified(bytes32 indexed blockHash);

    /// @notice Method used to commit an L2 block to be proved.
    /// @dev This method is used by the operator when a block is ready to be
    /// proved.
    /// @param previousBlockCommitment is the committment to the previous block.
    /// @param currentBlockCommitment is the committment to the block to be proved.
    function commit(bytes32 previousBlockCommitment, bytes32 currentBlockCommitment) external;

    /// @notice Method used to verify an L2 block proof.
    /// @dev This method is used by the operator when a block is ready to be
    /// verified (this is after proved).
    /// @param blockProof is the proof of the block to be verified.
    function verify(bytes calldata blockProof) external;
}
