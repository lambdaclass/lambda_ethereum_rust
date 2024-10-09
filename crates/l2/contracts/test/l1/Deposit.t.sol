// SPDX-License-Identifier: MIT

pragma solidity 0.8.27;

import {CommonBridgeTest} from "./CommonBridge.t.sol";

contract DepositTest is CommonBridgeTest {
    event DepositInitiated(uint256 indexed amount, address indexed to, bytes32 indexed l2MintTxHash);

    error AmountToDepositIsZero();

    function test_cannotDepositWithAmountZero() public {
        vm.expectRevert(abi.encodePacked(AmountToDepositIsZero.selector));
        commonBridge.deposit{value: 0 ether}(alice);
    }

    function test_cannotDepositThroughEOATransferWithAmountZero() public {
        vm.expectRevert(abi.encodePacked(AmountToDepositIsZero.selector));
        commonBridge.deposit{value: 0 ether}(alice);
    }

    function test_canDeposit() public {
        vm.expectEmit(true, true, true, true, address(commonBridge));
        emit DepositInitiated(0.1 ether, alice, dummyl2MintTxHash);
        vm.deal(alice, 1 ether);
        commonBridge.deposit{value: 0.1 ether}(alice);
    }

    function test_canDepositThroughEOATransfer() public {
        vm.expectEmit(true, true, true, true, address(commonBridge));
        emit DepositInitiated(0.1 ether, alice, dummyl2MintTxHash);
        vm.deal(alice, 1 ether);
        (bool success,) = address(commonBridge).call{value: 0.1 ether}("");
        require(success, "Transfer failed");
    }
}
