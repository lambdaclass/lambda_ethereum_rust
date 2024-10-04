#! /usr/bin/node

// Chop stacktop to be no more than two bytes. This lets us ensure
// opcodes that need a low parameter value get it.
//
// PUSH2 FFFF
// AND
const chop2Bytes = "61FFFF16"

// Chop to one byte
// PUSH1 FF
// AND
const chop1Byte = "60FF16"


// Chop the top two values into two byte values
// This is useful when we have an address and a buffer length
// PUSH2 FFFF
// AND
// SWAP1
// PUSH2 FFFF
// AND
const chop2Val2Bytes = "61FFFF169061FFFF16"


// Push a random 2 byte value
const pushRand2 = "61xxxx"

// Push a random big (32 byte) value
let pushRandBig = "7F"

for(var i=0; i<32; i++)
  pushRandBig += "xx"

// This string accumulates a description of what the code fragment does,
// to facilitate understanding WHY it failed

let progDesc = ""


// This table contains code fragments and the size of their
// input and output. These fragments may be more than a single
// opcode long because some opcodes, such as JUMP[I], don't
// do well without a special environment.
//
// Where xx appears in a fragment it means "just put a randon
// value here".
//
// The initial table only contains the unusual cases. Everything else is
// added by calls later

let frags = [
    {
        // POP
        frag: "50",
        in: 1,
        out: 0,
        desc: "# POP"
    },
    {
        // SSTORE
        frag: "55",
        in: 2,
        out: 0,
        desc: "# SSTORE"
    },
    {
        // JUMPDEST
        frag: "5B",
        in: 0,
        out: 0,
        desc: "# JUMPDEST"
    },
    {   // BLOCKHASH              Stacktop
        frag: chop1Byte +  //    <1 byte val>
              "43" + // NUMBER   <current block #>   <1 byte val>
              "03" + // SUB      <current block # - 1 byte val>
              "40",  // BLOCKHASH    <Number of recent block>
        in: 1,
        out: 1,
        desc: `#
#     BLOCKHASH:
# PUSH1 0xFF
# AND
# NUMBER
# SUB
# BLOCKHASH
#`
    },


    // Memory opcodes, can't have huge buffers or high addresses if we want to
    // avoid OOG
    {   // MLOAD
        frag: chop2Bytes + "51",
        in: 1,
        out: 1,
        desc: `#
#             MLOAD
# PUSH2 0xFFFF
# AND
# MLOAD
#`
    },
    {   // MSTORE, can have any value but address needs to be reasonable
        frag: chop2Bytes + "52",
        in: 2,
        out: 0,
        desc: `#
#             MSTORE
# PUSH2 0xFFFF
# AND
# MSTORE
#`
    },
    {   // MSTORE8, same as MSTORE
        frag: chop2Bytes + "53",
        in: 2,
        out: 0,
        desc: `#
#             MSTORE8
# PUSH2 0xFFFF
# AND
# MLOAD
#`
    },
    {   // SHA3
        frag: chop2Val2Bytes + "20",
        in: 2,
        out: 1,
        desc: `#
#        SHA3
# PUSH2 FFFF
# AND
# SWAP1
# PUSH2 FFFF
# AND
# SHA3
#`
    },
    {   // CALLDATACOPY        Stacktop \/
        frag: chop2Val2Bytes + // <2 byte value>, <2 byte value>, <big value>
           "91" +   // SWAP2      <big value>, <2 byte value>, <2 byte value>
           "90" +   // SWAP1      <2 byte value>, <big value>, <2 byte value>
           "37",    // CALLDATACOPY, <mem addr>, <calldata addr>, <length>
        in: 3,
        out: 0,
        desc: `#
#     CALLDATACOPY
# PUSH2 FFFF
# AND
# SWAP1
# PUSH2 FFFF
# AND
# SWAP2
# SWAP1
# CALLDATACOPY
#`
    },  // CODECOPY, same logic as CALLDATACOPY
    {
        frag: chop2Val2Bytes + "919039",
        in: 3,
        out: 0,
        desc: `#
#     CODECOPY
# PUSH2 FFFF
# AND
# SWAP1
# PUSH2 FFFF
# AND
# SWAP2
# SWAP1
# CODECOPY
#`
    },
    {   // EXTCODECOPY             Stacktop \/
        frag: chop2Val2Bytes + //  <2 byte val>, <2 byte val>, <big val>,   <big val>
           "92" + // SWAP3         <big val>,    <2 byte val>, <big val>,   <2 byte val>
           "3C",  // EXTCODECOPY   <contract>    <mem addr>    <code addr>  <length>
        in: 4,
        out: 0,
        desc: `#
#     EXTCODECOPY
# PUSH2 FFFF
# AND
# SWAP1
# PUSH2 FFFF
# AND
# SWAP2
# SWAP1
# EXTCODECOPY
#`
    },
    {   // CREATE               Stacktop \/
        frag: chop2Val2Bytes +  // <2 byte val>  <2 byte val>   <big val>
           "91" +    // SWAP2      <big val>     <2 byte val>   <2 byte val>
           chop2Bytes +         // <2 byte val>  <2 byte val>   <2 byte val>
           "F0",     // CREATE      val (wei)      <mem addr>    <length>
        in: 3,
        out: 1,
        desc: `#
#     CREATE
# PUSH2 FFFF
# AND
# SWAP1
# PUSH2 FFFF
# AND
# SWAP2
# SWAP1
# SWAP2
# PUSH2 FFFF
# AND
# CREATE
#`
    },
    {   // CREATE2              Stacktop \/
        frag: chop2Val2Bytes +  // <2 byte val>  <2 byte val>   <big val>    <big val>
           "91" +    // SWAP2      <big val>     <2 byte val>   <2 byte val> <big val>
           chop2Bytes +         // <2 byte val>  <2 byte val>   <2 byte val> <big val>
           "F5",     // CREATE2     val (wei)      <mem addr>    <length>      <salt>
        in: 4,
        out: 1,
        desc: `#
#     CREATE2
# PUSH2 FFFF
# AND
# SWAP1
# PUSH2 FFFF
# AND
# SWAP2
# SWAP1
# SWAP2
# PUSH2 FFFF
# AND
# CREATE2
#`
    },
    {   // BLOCKHASH              Stacktop
        frag: chop1Byte +  //    <1 byte val>
              "43" + // NUMBER   <current block #>   <1 byte val>
              "03" + // SUB      <current block # - 1 byte val>
              "40",  // BLOCKHASH    <Number of recent block>
        in: 1,
        out: 1,
        desc: `#
#     BLOCKHASH
# PUSH1 FF
# AND
# NUMBER
# SUB
# BLOCKHASH
#`
    },

    // CALL opcodes. Managing the stack is too much pain, so we just
    // push random values ourselves
    {   // CALL
        frag: pushRand2 +   // retLen
              pushRand2 +   // ret mem addr
              pushRand2 +   // argLen
              pushRand2 +   // arg mem addr
              pushRand2 +   // val
              pushRandBig + // addr
              "5A" +        // GAS (everything we have)
              "F1",         // CALL
        in: 0,   // since we create our own
        out: 1,  // return value
        desc: `#
#     CALL
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH32 <random value>
# GAS
# CALL
#`
    },
    {   // CALLCODE
        frag: pushRand2 +   // retLen
              pushRand2 +   // ret mem addr
              pushRand2 +   // argLen
              pushRand2 +   // arg mem addr
              pushRand2 +   // val
              pushRandBig + // addr
              "5A" +        // GAS (everything we have)
              "F2",         // CALLCODE
        in: 0,   // since we create our own
        out: 1,  // return value
        desc: `#
#     CALLCODE
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH32 <random value>
# GAS
# CALLCODE
#`
    },
    {   // DELEGATECALL
        frag: pushRand2 +   // retLen
              pushRand2 +   // ret mem addr
              pushRand2 +   // argLen
              pushRand2 +   // arg mem addr
              pushRandBig + // addr
              "5A" +        // GAS (everything we have)
              "F4",         // DELEGATECALL
        in: 0,   // since we create our own
        out: 1,  // return value
        desc: `#
#     DELEGATECALL
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH32 <random value>
# GAS
# DELEGATECALL
#`
    },
    {   // STATICCALL
        frag: pushRand2 +   // retLen
              pushRand2 +   // ret mem addr
              pushRand2 +   // argLen
              pushRand2 +   // arg mem addr
              pushRandBig + // addr
              "5A" +        // GAS (everything we have)
              "FA",         // STATICCALL
        in: 0,   // since we create our own
        out: 1,  // return value
        desc: `#
#     STATICCALL
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH2 <random value>
# PUSH32 <random value>
# GAS
# STATICCALL
#`
    },

    // JUMPs. Need to always jump to a JUMPDEST
    {   // JUMP                       Stacktop
        frag: "58"   +    // 00 PC        <frag start>
              "6006" +    // 01 PUSH1 6   6     <frag start>
              "01"   +    // 03 ADD       <frag start + 6>
              "56"   +    // 04 JUMP        to this addr in code
              "xx"   +    // 05 nonsense
              "5B",       // 06 JUMPDEST
        in: 0,
        out: 0,
        desc: `#
#         JUMP
# PC
# PUSH1 06
# ADD
# JUMP
# <random byte we don't use>
# JUMPDEST
#`
    },
    {   // JUMPI                           Stacktop
        frag: "6001" +    //    PUSH1  1   1 <input val>
              "16"   +    //    AND        <input val & 1>  we jump in half the cases

              // We count the fragment from here, the point we get the PC
              "58"   +    // 00 PC        <frag start>        <input val & 1>
              "6008" +    // 01 PUSH1 8   8                   <frag start>  <input val & 1>
              "01"   +    // 03 ADD       <frag start + 8>    <input val & 1>
              "57"   +    // 04 JUMPI   to this addr in code  if this isn't zero
              "60xx" +    // 05 PUSH1 <random>
              "50"   +    // 07 POP
              "5B",       // 08 JUMPDEST
        in: 1,
        out: 0,
        desc: `#
#         JUMPI
# PUSH1 01
# AND
# PC
# PUSH1 08
# ADD
# JUMPI
# PUSH1 <random>
# POP
# JUMPDEST
#`
    }
]

// For a single byte value
const int2str = val => val.toString(16).padStart(2, "0")


// Opcode table, for the opcodes that aren't exceptions
const opcodes = {
  "01": "ADD",
  "02": "MUL",
  "03": "SUB",
  "04": "DIV",
  "05": "SDIV",
  "06": "MOD",
  "07": "SMOD",
  "08": "ADDMOD",
  "09": "MULMOD",
  "0a": "EXP",
  "0b": "SIGNEXTEND",
  "10": "LT",
  "11": "GT",
  "12": "SLT",
  "13": "SGT",
  "14": "EQ",
  "15": "ISZERO",
  "16": "AND",
  "17": "OR",
  "18": "XOR",
  "19": "NOT",
  "1a": "BYTE",
  "1b": "SHL",
  "1c": "SHR",
  "1d": "SAR",
  "30": "ADDRESS",
  "31": "BALANCE",
  "32": "ORIGIN",
  "33": "CALLER",
  "34": "CALLVALUE",
  "35": "CALLDATALOAD",
  "36": "CALLDATASIZE",
  "38": "CODESIZE",
  "3a": "GASPRICE",
  "3b": "EXTCODESIZE",
  "3d": "RETURNDATASIZE",
  "3f": "EXTCODEHASH",
  "41": "COINBASE",
  "42": "TIMESTAMP",
  "43": "NUMBER",
  "44": "DIFFICULTY",
  "45": "GASLIMIT",
  "46": "CHAINID",
  "47": "SELFBALANCE",
  "48": "BASEFEE",
  "54": "SLOAD",
  "58": "PC",
  "59": "MSIZE",
  "5a": "GAS"
}


// A one value to one value opcode, such as NOT etc.
const one2one = opc => {
    frags.push({
        frag: int2str(opc),
        in: 1,
        out: 1,
        desc: `# ${opcodes[int2str(opc)]}`
    })
  }

// A two value to one value opcode, such as ADD, MUL, etc.
const two2one = opc => {
  frags.push({
        frag: int2str(opc),
        in: 2,
        out: 1,
        desc: `# ${opcodes[int2str(opc)]}`
  })
}

// A three value to one value opcode, such as ADDMOD, MULMOD, etc.
const three2one = opc => {
    frags.push({
        frag: int2str(opc),
        in: 3,
        out: 1,
        desc: `# ${opcodes[int2str(opc)]}`
    })
  }


// Reading a state value, 0->1
const readState = opc => {
    frags.push({
        frag: int2str(opc),
        in: 0,
        out: 1,
        desc: `# ${opcodes[int2str(opc)]}`
    })
  }


for (var i=1; i<=7; i++)
  two2one(i)

three2one(0x08) // ADDMOD
three2one(0x09) // MULMOD
two2one(0x0A)   // EXP
two2one(0x0B)   // SIGNEXTEND

for (var i=0x10; i<=0x14; i++)
  two2one(i)

one2one(0x15)  // ISZERO

for (var i=0x16; i<=0x18; i++)
  two2one(i)

one2one(0x19)  // NOT

for (var i=0x1A; i<=0x1D; i++)
  two2one(i)


readState(0x30)   // ADDRESS
one2one(0x31)     // BALANCE

for (var i=0x32; i<=0x34; i++)
  readState(i)


one2one(0x35)   // CALLDATALOAD
readState(0x36) // CALLDATASIZE
readState(0x38) // CODESIZE
readState(0x3A) // GASPRICE
one2one(0x3B)   // EXTCODESIZE
readState(0x3D) // RETURNDATASIZE
one2one(0x3F)   // EXTCODEHASH


for (var i=0x41; i<=0x48; i++)
  readState(i)

// 0x50 POP is a special case, specifically defined in the table

one2one(0x54)  // SLOAD

// 0x55 SSTORE is specifically defined in the table


for (var i=0x58; i<=0x5A; i++)
  readState(i)


// PUSHn
for (var i=0; i<=0x1F; i++)
  frags.push({
    frag: int2str(0x60+i) + "xx".repeat(i+1),
    in: 0,
    out: 1,
    desc: `# PUSH${i+1} <random value>`
  })


// DUPn
for (var i=0; i<=0x0F; i++)
  frags.push({
    frag: int2str(0x80+i),
    in: 1+i,
    out: 2+i,  // doesn't consume the input, adds one output
    desc: `# DUP${i+1}`
  })


// SWAPn
for (var i=0; i<=0x0F; i++)
  frags.push({
    frag: int2str(0x90+i),
    in: 2+i,
    out: 2+i, // doesn't consume the input
    desc: `# SWAP${i+1}`
  })


// LOGn, the top two parameters are memory (addr and length)
for (var i=0; i<=4; i++)
  frags.push({
    frag: chop2Val2Bytes + int2str(0xA0+i),
    in: 2+i,
    out: 0,
    desc: `# LOG${i}`
  })


// Terminal opcodes. By the time we get to them the stack is
// empty
const terminalFrags = [
  "00",                         // STOP
  pushRand2 + pushRand2 + "F3", // RETURN
  pushRand2 + pushRand2 + "FD"  // REVERT
]  // terminal frags



// Get a random element
const getRandom = list => list[Math.floor((Math.random()*list.length))]


// Select a fragment, but only if it can run with this stack depth
const selectFrag = stackDepth => {
    frag = getRandom(frags)
    while(frag.in > stackDepth)
        frag = getRandom(frags)

    return frag;
}

// Get a whole program that won't suffer from a stack underflow. The
// length is in fragments, the number of opcodes and/or bytes will likely
// be higher
const getProg = length => {
    var stackDepth = 0
    var prog = ""
    var desc = ""

    // Get the program
    for(var i=0; i<length; i++) {
        const frag = selectFrag(stackDepth)
        prog += frag.frag
        desc += "\n" + frag.desc
        stackDepth = stackDepth - frag.in + frag.out
    }

    // Write the stack to storage, so we'll be able to identify if two
    // clients differ
    for(i=0; i<stackDepth; i++)
      prog += `60${int2str(i)}55`

    // Terminal fragment
    prog += getRandom(terminalFrags)

    // Fill the random values
    while(prog.match("x"))
      prog = prog.replace("x", getRandom("0123456789ABCDEF"))

    return [prog, desc]
}

const progAndDesc = getProg(100)
const fillerYML = `
# Created by tests/src/Templates/Random/randomTest.js

random:
  _info:
    comment: Ori Pomerantz   qbzzt1@gmail.com

  env:
    currentCoinbase: 2adc25665018aa1fe0e6bc666dac8fc2697ff9ba
    currentDifficulty: 0x20000
    currentGasLimit: 0xFF112233445566
    currentNumber: 1
    currentTimestamp: 1000

  pre:

    # Perform the random action
    cccccccccccccccccccccccccccccccccccccccc:
      balance: 1000000000000000000
      code: :raw 0x${progAndDesc[0]}
      nonce: 1
      storage: {}

    # Explain the random action
${progAndDesc[1]}



    a94f5374fce5edbc8e2a8697c15331677e6ebf0b:
      balance: 1000000000000000000000
      code: '0x'
      nonce: 1
      storage: {}


  transaction:
    data:
    - 0x
    gasLimit:
    - 0x10000000000000
    nonce: 1
    to: cccccccccccccccccccccccccccccccccccccccc
    value:
    - 0
    secretKey: "45a915e4d060149eb4365960e6a7a45f334393093061116b197e3240065ff2d8"
    gasPrice: 100


  expect:
    - indexes:
        data: !!int -1
        gas:  !!int -1
        value: !!int -1

      network:
        - '>=London'
      result: {}
`   // fillerYML


console.log(fillerYML)
