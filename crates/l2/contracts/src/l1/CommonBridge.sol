// SPDX-License-Identifier: MIT
pragma solidity 0.8.27;

import "./interfaces/ICommonBridge.sol";

/// @title CommonBridge contract.
/// @author LambdaClass
contract CommonBridge is ICommonBridge {
    /// @inheritdoc ICommonBridge
    function deposit(address to) public payable {
        if (msg.value == 0) {
            revert AmountToDepositIsZero();
        }
        // TODO: Build the tx.
        bytes32 l2MintTxHash = keccak256(abi.encodePacked("dummyl2MintTxHash"));
        emit DepositInitiated(msg.value, to, l2MintTxHash);
    }

    receive() external payable {
        deposit(msg.sender);
    }
}
