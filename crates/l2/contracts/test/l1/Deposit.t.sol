// SPDX-License-Identifier: MIT

pragma solidity 0.8.27;

import {CommonBridgeTest} from "./CommonBridge.t.sol";

contract DepositTest is CommonBridgeTest {
    event DepositInitiated(bytes32 indexed l2MintTxHash);

    error AmountToDepositIsZero();

    function test_cannotDepositWithAmountZero() public {
        vm.expectRevert(abi.encodePacked(AmountToDepositIsZero.selector));
        commonBridge.deposit{value: 0 ether}(alice, alice);
    }

    function test_depositSuccessfully() public {
        vm.expectEmit(true, true, true, true, address(commonBridge));
        emit DepositInitiated(dummyl2MintTxHash);
        vm.deal(alice, 1 ether);
        commonBridge.deposit{value: 0.1 ether}(alice, alice);
    }
}
