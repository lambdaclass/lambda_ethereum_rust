// SPDX-License-Identifier: MIT
pragma solidity 0.8.27;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "./interfaces/ICommonBridge.sol";

/// @title CommonBridge contract.
/// @author LambdaClass
contract CommonBridge is ICommonBridge, Ownable, ReentrancyGuard {
    /// @notice Mapping of unclaimed withdrawals. A withdrawal is claimed if
    /// there is a non-zero value in the mapping (a merkle root) for the hash
    /// of the L2 transaction that requested the withdrawal.
    /// @dev The key is the hash of the L2 transaction that requested the
    /// withdrawal.
    /// @dev The value is a boolean indicating if the withdrawal was claimed or not.
    mapping(bytes32 => bool) public claimedWithdrawals;

    /// @notice Mapping of merkle roots to the L2 withdrawal transaction logs.
    /// @dev The key is the L2 block number where the logs were emitted.
    /// @dev The value is the merkle root of the logs.
    /// @dev If there exist a merkle root for a given block number it means
    /// that the logs were published on L1, and that that block was committed.
    mapping(uint256 => bytes32) public blockWithdrawalsLogs;

    address public ON_CHAIN_PROPOSER;

    modifier onlyOnChainProposer() {
        require(
            msg.sender == ON_CHAIN_PROPOSER,
            "CommonBridge: caller is not the OnChainProposer"
        );
        _;
    }

    constructor(address owner) Ownable(owner) {}

    function initialize(address onChainProposer) public nonReentrant {
        require(
            onChainProposer != address(0),
            "CommonBridge: onChainProposer is the zero address"
        );
        require(
            onChainProposer != address(this),
            "CommonBridge: onChainProposer is the contract address"
        );
        ON_CHAIN_PROPOSER = onChainProposer;
    }

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
    function publishWithdrawals(
        uint256 withdrawalLogsBlockNumber,
        bytes32 withdrawalsLogsMerkleRoot
    ) public onlyOnChainProposer {
        require(
            blockWithdrawalsLogs[withdrawalLogsBlockNumber] == bytes32(0),
            "CommonBridge: withdrawal logs already published"
        );
        blockWithdrawalsLogs[
            withdrawalLogsBlockNumber
        ] = withdrawalsLogsMerkleRoot;
        emit WithdrawalsPublished(
            withdrawalLogsBlockNumber,
            withdrawalsLogsMerkleRoot
        );
    }

    /// @inheritdoc ICommonBridge
    function claimWithdrawal(
        bytes32 l2WithdrawalTxHash,
        uint256 claimedAmount,
        uint256 withdrawalBlockNumber,
        bytes32[] calldata //withdrawalProof
    ) public nonReentrant {
        require(
            blockWithdrawalsLogs[withdrawalBlockNumber] != bytes32(0),
            "CommonBridge: the block that emitted the withdrawal logs was not committed"
        );
        require(
            claimedWithdrawals[l2WithdrawalTxHash] == false,
            "CommonBridge: the withdrawal was already claimed"
        );
        // TODO: Verify the proof.
        require(true, "CommonBridge: invalid withdrawal proof");

        (bool success, ) = payable(msg.sender).call{value: claimedAmount}("");

        require(success, "CommonBridge: failed to send the claimed amount");

        claimedWithdrawals[l2WithdrawalTxHash] = true;

        emit WithdrawalClaimed(l2WithdrawalTxHash, msg.sender, claimedAmount);
    }
}
