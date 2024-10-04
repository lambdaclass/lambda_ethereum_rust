
#! /usr/bin/bash

# The 0xEn opcode block requires EOF, so it needs to be tested by other means


# Create the opcode tests
./testOpcodesGen.js 0x00 0x0F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_00Filler.yml
./testOpcodesGen.js 0x10 0x1F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_10Filler.yml
./testOpcodesGen.js 0x20 0x2F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_20Filler.yml
./testOpcodesGen.js 0x30 0x3F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_30Filler.yml
./testOpcodesGen.js 0x40 0x4F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_40Filler.yml
./testOpcodesGen.js 0x50 0x5F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_50Filler.yml
./testOpcodesGen.js 0x60 0x6F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_60Filler.yml
./testOpcodesGen.js 0x70 0x7F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_70Filler.yml
./testOpcodesGen.js 0x80 0x8F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_80Filler.yml
./testOpcodesGen.js 0x90 0x9F > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_90Filler.yml
./testOpcodesGen.js 0xa0 0xaF > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_a0Filler.yml
./testOpcodesGen.js 0xb0 0xbF > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_b0Filler.yml
./testOpcodesGen.js 0xc0 0xcF > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_c0Filler.yml
./testOpcodesGen.js 0xd0 0xdF > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_d0Filler.yml
# ./testOpcodesGen.js 0xe0 0xeF > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_e0Filler.yml
./testOpcodesGen.js 0xf0 0xfF > ../../BlockchainTestsFiller/ValidBlocks/bcStateTests/testOpcode_f0Filler.yml
