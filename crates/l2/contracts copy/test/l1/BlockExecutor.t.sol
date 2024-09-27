// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {BlockExecutor} from "../../src/l1/BlockExecutor.sol";

contract BlockExecutorTest is Test {
    BlockExecutor public blockExecutor;

    function setUp() public {
        blockExecutor = new BlockExecutor();
    }
}
