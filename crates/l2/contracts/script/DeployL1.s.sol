// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {BlockExecutor} from "../src/l1/BlockExecutor.sol";
import {Inbox} from "../src/l1/Inbox.sol";
import {Utils} from "./Utils.sol";

contract DeployL1Script is Script {
    address constant DETERMINISTIC_CREATE2_ADDRESS = 0x4e59b44847b379578588920cA78FbF26c0B4956C;
    address internal create2Factory;

    function setUp() public {
        instantiateCreate2Factory();
    }

    function run() public {
        console.log("Deploying L1 contracts");

        deployBlockExecutor();
        deployInbox();
    }

    function instantiateCreate2Factory() internal {
        address contractAddress;

        bool isDeterministicDeployed = DETERMINISTIC_CREATE2_ADDRESS.code.length > 0;

        if (isDeterministicDeployed) {
            contractAddress = DETERMINISTIC_CREATE2_ADDRESS;
            console.log("Using deterministic Create2Factory address:", contractAddress);
        } else {
            contractAddress = Utils.deployCreate2Factory();
            console.log("Create2Factory deployed at:", contractAddress);
        }
        create2Factory = contractAddress;
    }

    function deployBlockExecutor() internal {
        bytes memory bytecode = type(BlockExecutor).creationCode;
        bytes32 salt = bytes32(0);
        address contractAddress = Utils.deployWithCreate2(bytecode, salt, create2Factory);
        console.log("BlockExecutor deployed at:", contractAddress);
    }

    function deployInbox() internal {
        bytes memory bytecode = type(Inbox).creationCode;
        bytes32 salt = bytes32(0);
        address contractAddress = Utils.deployWithCreate2(bytecode, salt, create2Factory);
        console.log("Inbox deployed at:", contractAddress);
    }
}
