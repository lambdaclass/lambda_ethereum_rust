// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {OnChainProposer, IOnChainProposer} from "../src/l1/OnChainProposer.sol";
import {CommonBridge, ICommonBridge} from "../src/l1/CommonBridge.sol";
import {Utils} from "./Utils.sol";

contract DeployL1Script is Script {
    /// @notice Address of the deterministic create2 factory.
    /// @dev This address corresponds to a contracts that is set in the storage
    /// in the genesis file. The same contract with the same address is deployed
    /// in every testnet, so if this script is run in a testnet instead of in a
    /// local environment, it should work.
    address constant DETERMINISTIC_CREATE2_ADDRESS =
        0x4e59b44847b379578588920cA78FbF26c0B4956C;

    function setUp() public {}

    function run() public {
        console.log("Deploying L1 contracts");

        bytes32 salt = bytes32(0);

        address onChainProposer = deployOnChainProposer(salt);
        address commonBridge = deployCommonBridge(msg.sender, salt);

        vm.startBroadcast();
        IOnChainProposer(onChainProposer).initialize(commonBridge);
        ICommonBridge(commonBridge).initialize(onChainProposer);
        vm.stopBroadcast();
    }

    function deployOnChainProposer(bytes32 salt) internal returns (address) {
        bytes memory bytecode = type(OnChainProposer).creationCode;
        address contractAddress = Utils.deployWithCreate2(
            bytecode,
            salt,
            DETERMINISTIC_CREATE2_ADDRESS,
            ""
        );
        console.log("OnChainProposer deployed at:", contractAddress);
        return contractAddress;
    }

    function deployCommonBridge(
        address owner,
        bytes32 salt
    ) internal returns (address) {
        bytes memory bytecode = type(CommonBridge).creationCode;
        address contractAddress = Utils.deployWithCreate2(
            bytecode,
            salt,
            DETERMINISTIC_CREATE2_ADDRESS,
            abi.encode(owner)
        );
        console.log("CommonBridge deployed at:", contractAddress);
        return contractAddress;
    }
}
