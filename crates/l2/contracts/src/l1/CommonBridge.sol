// SPDX-License-Identifier: MIT
pragma solidity 0.8.27;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/ICommonBridge.sol";

/// @title CommonBridge contract.
/// @author LambdaClass
contract CommonBridge is ICommonBridge, Ownable {
    constructor(address owner) Ownable(owner) {}

    struct WithdrawalData {
        address to;
        uint256 amount;
    }

    mapping(bytes32 l2TxHash => WithdrawalData) public pendingWithdrawals;

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

    /// @inheritdoc ICommonBridge
    function startWithdrawal(
        WithdrawalTransaction[] calldata transactions
    ) public onlyOwner {
        for (uint256 i = 0; i < transactions.length; i++) {
            pendingWithdrawals[transactions[i].l2TxHash] = WithdrawalData(
                transactions[i].to,
                transactions[i].amount
            );
        }
    }

    function finalizeWithdrawal(bytes32 l2TxHash) public {
        require(
            msg.sender == pendingWithdrawals[l2TxHash].to,
            "CommonBridge: withdrawal not found"
        );

        payable(msg.sender).transfer(pendingWithdrawals[l2TxHash].amount);
        pendingWithdrawals[l2TxHash] = WithdrawalData(address(0), 0);
    }
}
