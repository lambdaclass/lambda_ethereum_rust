// SPDX-License-Identifier: MIT
pragma solidity 0.8.27;

import "./interfaces/IBlockExecutor.sol";

/// @title BlockExecutor contract.
/// @author LambdaClass
contract BlockExecutor is IBlockExecutor {
    /// @inheritdoc IBlockExecutor
    function commit(bytes32 currentBlockCommitment) external override {
        emit BlockCommitted(currentBlockCommitment);
    }

    /// @inheritdoc IBlockExecutor
    function verify(bytes calldata blockProof) external override {
        bytes32 blockHash = keccak256(abi.encode(blockProof));
        emit BlockVerified(blockHash);
    }
}
