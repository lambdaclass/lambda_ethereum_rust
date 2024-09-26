// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Inbox} from "../../src/l1/Inbox.sol";

contract InboxTest is Test {
    Inbox public inbox;

    function setUp() public {
        inbox = new Inbox();
    }
}
