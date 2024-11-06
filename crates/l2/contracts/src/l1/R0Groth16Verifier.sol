// Copyright 2024 RISC Zero, Inc.
//
// The RiscZeroGroth16Verifier is a free software: you can redistribute it
// and/or modify it under the terms of the GNU General Public License as
// published by the Free Software Foundation, either version 3 of the License,
// or (at your option) any later version.
//
// The RiscZeroGroth16Verifier is distributed in the hope that it will be
// useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
// Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// the RiscZeroGroth16Verifier. If not, see <https://www.gnu.org/licenses/>.
//
// Changes in risc0-ethereum/contracts/src/StructHash.sol:
// - import {SafeCast} from "openzeppelin/contracts/utils/math/SafeCast.sol";
// + import {SafeCast} from "../../../openzeppelin-contracts/contracts/utils/math/SafeCast.sol";
// Changes in risc0-ethereum/contracts/src/groth16/RiscZeroGroth16Verifier.sol:
// - import {SafeCast} from "openzeppelin/contracts/utils/math/SafeCast.sol";
// + import {SafeCast} from "../../../../openzeppelin-contracts/contracts/utils/math/SafeCast.sol";
// SPDX-License-Identifier: GPL-3.0

pragma solidity ^0.8.27;

import "../../lib/risc0-ethereum/contracts/src/groth16/RiscZeroGroth16Verifier.sol";
import {ControlID} from "../../lib/risc0-ethereum/contracts/src/groth16/ControlID.sol";

/// @title R0Groth16Verifier contract.
///
contract R0Groth16Verifier is RiscZeroGroth16Verifier {
    constructor()
        RiscZeroGroth16Verifier(
            ControlID.CONTROL_ROOT,
            ControlID.BN254_CONTROL_ID
        )
    {}
}
