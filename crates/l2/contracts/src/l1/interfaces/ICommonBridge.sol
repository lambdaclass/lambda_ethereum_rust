// SPDX-License-Identifier: MIT
pragma solidity 0.8.27;

/// @title Interface for the CommonBridge contract.
/// @author LambdaClass
/// @notice A CommonBridge contract is a contract that allows L1<->L2 communication
/// from L1. It both sends messages from L1 to L2 and receives messages from L2.
interface ICommonBridge {
    /// @notice A deposit to L2 has initiated.
    /// @dev Event emitted when a deposit is initiated.
    /// @param amount the amount of tokens being deposited.
    /// @param to the address in L2 to which the tokens will be minted to.
    /// @param l2MintTxHash the hash of the transaction that will finalize the
    /// deposit in L2. Could be used to track the status of the deposit finalization
    /// on L2. You can use this hash to retrive the tx data.
    /// It is the result of keccak(abi.encode(transaction)).
    event DepositInitiated(uint256 indexed amount, address indexed to, bytes32 indexed l2MintTxHash);

    /// @notice Error for when the deposit amount is 0.
    error AmountToDepositIsZero();
    
    /// @notice Method that starts an L2 ETH deposit process.
    /// @dev The deposit process starts here by emitting a DepositInitiated
    /// event. This event will later be intercepted by the L2 operator to
    /// finalize the deposit.
    /// @param to, the address in L2 to which the tokens will be minted to.
    function deposit(address to) external payable;
}
