// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {CommonBridge} from "../../src/l1/CommonBridge.sol";

contract CommonBridgeTest is Test {
    CommonBridge internal commonBridge;
    address internal alice;
    bytes32 internal dummyl2MintTxHash;

    constructor() {
        commonBridge = new CommonBridge();
        alice = makeAddr("alice");
        dummyl2MintTxHash = keccak256(abi.encodePacked("dummyl2MintTxHash"));
    }
}
