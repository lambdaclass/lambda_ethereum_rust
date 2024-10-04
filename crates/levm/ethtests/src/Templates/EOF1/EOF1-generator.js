#! /usr/bin/node

// Create EOF1 code, with values that are valid and invalid in various fields.
// based on https://github.com/lightclient/go-ethereum/blob/eof/core/vm/eof.go
// Then create a test with deploy transactions for all of them, and see that
// only the valid ones result in contracts

// Values for good eof1 code
// This is the prototype that other examples, valid and invalid, build on
const eof1Good = {
  magic: 0xEF00,
  version: 1,
  sections: [
    {    // EIP 4750 section header
       type: 1,  // EIP 4750 metadata
       size: 4   // four bytes (per code section)
    },
    {    // code sections
       type: 2,   // Code sections
       size: 1,   // One code section
       lengths: [3]   // Length of that code section
    },
    {    // data section
       type: 4,   // Data section
       size: 1    // One byte
    }
  ],    // sections
  endOfSections: 0,  // Specified so we can specify a wrong value
  code: [
      {
         code: "305000",
         stackInputs: 0,
         stackOutputs: 128, // 0x80 (Non-Returning Function)
         maxStack: 1      // maxStack needs to be accurate, otherwise CREATE[2] fails
      }
  ],
  data: "ef"
}


// Create a one byte value
const byte1 = val => val.toString(16).padStart(2,0).slice(0,2)

// Create a two byte value
const byte2 = val => val.toString(16).padStart(4,0).slice(0,4)


// Convert a specific section into the EOF1 format
const section2eof1 = section => {
  switch (section.type) {
    case 2:   // Code sections, of which there could be many
      return byte1(section.type) + byte2(section.size) +
          section.lengths.map(byte2).reduce((a,b) => a+b)
      // return handles break for us
    default:  // Including invalid values
      return byte1(section.type) + byte2(section.size)

  }
}    // section2eof1



// Create the EIP4750 section
const code2eip4750 = code =>
      byte1(code.stackInputs)+byte1(code.stackOutputs)+byte2(code.maxStack)


// Convert a hash table input EOF1 encoded value.
// No sanity checks, because the whole point is to be able to get invalid ones
const encode = hash => {
  // Start code
  res = byte2(hash.magic) + byte1(hash.version)

  // Section list
  res = res +
      hash.sections.map(section2eof1).reduce((a,b) => a+b) +
      byte1(hash.endOfSections)

  // Type section
  res = res + hash.code.map(code2eip4750).reduce((a,b) => a+b)

  // Finally the code sections and data section
  res = res + hash.code.map(code => code.code).reduce((a,b) => a+b) +
              hash.data

  return res
}  // encode


// Create a valid code section in an EOF1 structure
const createEOF1Code = (codeList, maxStacks) => {
    // Start from a valid copy
    eof1 = cloneEof1Good()

    // Size of sections
    eof1.sections[0].size = codeList.length*4
    eof1.sections[1].size = codeList.length

    // The values for each entry
    eof1.sections[1].lengths = codeList.map(code => code.length/2)

    // Assume stackInputs and stackOutputs are zero
    eof1.code = codeList.map((code, i) => { 
      let outputs = 128 // Assume is a Non-Returning Function 
      if ( i != 0 ) {
        // If code contains RETF opcode (0xe4), then it is a Returning Function
        for (var j = 0; j < code.length; j += 2) {
          if (code.slice(j, j+2) == "E4") {
            outputs = 0
            break
          }
        }
      }
      return {
	code: code,
	stackInputs: 0,
	stackOutputs: outputs,
	maxStack: maxStacks[i]
    }})

    return eof1
}  // createEOF1Code



// Make sure to do a deep copy so we can abuse it
const cloneEof1Good = () => JSON.parse(JSON.stringify(eof1Good))

eof1BadMagic = cloneEof1Good()
eof1BadMagic.magic = 0xEF02

eof1BadVersion0 = cloneEof1Good()
eof1BadVersion0.version = 0

eof1BadVersion2 = cloneEof1Good()
eof1BadVersion2.version = 2

eof1BadSectionOrder = cloneEof1Good()
eof1BadSectionOrder.sections = [
   eof1Good.sections[1],
   eof1Good.sections[2],
   eof1Good.sections[0]
]

eof1NoCodeSection = cloneEof1Good()
eof1NoCodeSection.sections = [
   eof1Good.sections[0],
   eof1Good.sections[2]
]

eof1NoDataSection = cloneEof1Good()
eof1NoDataSection.sections = [
   eof1Good.sections[0],
   eof1Good.sections[1]
]
eof1NoDataSection.data = ""

eof1BadEndOfSections = cloneEof1Good()
eof1BadEndOfSections.endOfSections = 255

eof1InconsistentCodeSec = cloneEof1Good()
eof1InconsistentCodeSec.sections[0].size = 8

eof1ExtraBytes = cloneEof1Good()
eof1ExtraBytes.data = "000bad"

eof1DataTooShort = cloneEof1Good()
eof1DataTooShort.sections[2].size = 6

eof1WierdSectionSize = cloneEof1Good()
eof1WierdSectionSize.sections[0].size = 3

eof1BadList = [
	[eof1BadMagic, "EOF1I0001", "EOF_InvalidPrefix"],
        [eof1BadVersion0, "EOF1I0002", "EOF_UnknownVersion"],
        [eof1BadVersion2, "EOF1I0002", "EOF_UnknownVersion"],
        [eof1BadSectionOrder, "EOF1I0003", "EOF_TypeSectionMissing"],
        [eof1NoCodeSection, "EOF1I0004", "EOF_CodeSectionMissing"],
        [eof1NoDataSection, "EOF1I0004", "EOF_DataSectionMissing"],
        [eof1BadEndOfSections, "EOF1I0005", "EOF_HeaderTerminatorMissing"],
        [eof1InconsistentCodeSec, "EOF1I0018", "EOF_InvalidSectionBodiesSize"],
        [eof1ExtraBytes, "EOF1I0006", "EOF_InvalidSectionBodiesSize"],
        [eof1WierdSectionSize, "", "EOF_InvalidSectionBodiesSize"]
]  // eof1BadList


eof1GoodList = [[eof1Good, "EOF1V0001", "ok."]]


// Multiple code segments are fine (only if all sections are called at least once)
eof1GoodList.push([createEOF1Code(["3050E3000100", "30305050E4"], [1, 2]), "EOF1V0002", "ok."])
eof1GoodList.push([createEOF1Code(["3050E3000100", "3050E30002E4", "3050E30003E4", "3050E4"],
                              [1,1,1,1]), "EOF1V0003", "ok."])

// Data is allowed to be shorter than specified
eof1GoodList.push([eof1DataTooShort, "EOF1V0016", "ok."])

// Create tests in groups. A good example, and bad examples that are related
// so we know they fail for the right reason.

// FE, the designated invalid opcode, is valid in a code segment.
// Other opcode values, however, are invalid
eof1GoodList.push([createEOF1Code(["FE"], [0]), "EOF1V0004", "ok."])
 eof1BadList.push([createEOF1Code(["EF"], [0]), "EOF1I0008", "EOF_UndefinedInstruction"])


// maxStack needs to be the correct value,
eof1GoodList.push([createEOF1Code(["3030505000"], [2]), "EOF1V0001", "ok."])
 eof1BadList.push([createEOF1Code(["3030505000"], [1]), "EOF1I0009", "EOF_InvalidMaxStackHeight"])
 eof1BadList.push([createEOF1Code(["3030505000"], [3]), "EOF1I0009", "EOF_InvalidMaxStackHeight"])

// stack underflow
 eof1BadList.push([createEOF1Code(["30505000"], [1]), "EOF1I0012", "EOF_StackUnderflow"])

// stack overflow
push10 = ""  // Push ten values into the stack
for (i=0; i<10; i++) push10 += "30"

push100 = "" // Push a hundred values into the stack
             // assume push10 is code segment 1
for(var i=0; i<10; i++) push100 += "E30001"

push1000 = "" // Push a thousand values into the stack
              // assume push100 is code segment 2
for(var i=0; i<10; i++) push1000 += "E30002"

// 1023 values into the stack
temp = createEOF1Code([push1000 + push10 + push10 + "303030" + "00",   // Segment 0
                        push10+"E4",                        // Segment 1
                        push100+"E4"],                      // Segment 2
                        [1023, 10, 100])   // maxStackHeight
// Specify how many outputs segments have
temp.code[1].stackOutputs = 10
temp.code[2].stackOutputs = 100
eof1GoodList.push([temp, "EOF1V0015", "ok."])


// 1024 values into the stack
temp = createEOF1Code([push1000 + push10 + push10 + "30303030" + "00",   // Segment 0
                        push10+"E4",                        // Segment 1
                        push100+"E4"],                      // Segment 2
                        [1024, 10, 100])   // maxStackHeight
// Specify how many outputs segments have
temp.code[1].stackOutputs = 10
temp.code[2].stackOutputs = 100
 eof1BadList.push([temp, "EOF1I0025", "EOF_MaxStackHeightExceeded"])



// parameters are allowed except on section 0
temp = createEOF1Code(["30E3000100", "300150E4"], [1, 2])
temp.code[1].stackInputs = 1
eof1GoodList.push([temp, "EOF1V0009", "ok."])

temp = createEOF1Code(["300150e3000100", "305000E4"], [2, 1])
temp.code[0].stackInputs = 1
 eof1BadList.push([temp, "EOF1I0017", "EOF_InvalidFirstSectionType"])


// Return values are allowed, except on section 0, which should be marked as
// Non-Returning Function (0x80)
temp = createEOF1Code(["3050E300015000", "303001E4"], [1, 2])
temp.code[1].stackOutputs = 1
eof1GoodList.push([temp, "EOF1V0006", "ok."])

temp = createEOF1Code(["303001E50001", "305000"], [2, 1])
temp.code[0].stackOutputs = 1
 eof1BadList.push([temp, "EOF1I0010", "EOF_InvalidFirstSectionType"])


// Function calls are allowed, as long as the function exists
//
 eof1BadList.push([createEOF1Code(["E3000200", "00"], [0, 0]), "EOF1I0011", "EOF_InvalidCodeSectionIndex"])


// Stack underflow is bad
eof1GoodList.push([createEOF1Code(["3030015000"], [2]), "", "ok."])
 eof1BadList.push([createEOF1Code(["30015000"], [1]), "EOF1I0012", "EOF_StackUnderflow"])


// Endless loops are OK
eof1GoodList.push([createEOF1Code(["3050E50000"], [1]), "EOF1V0007", "ok."])

// The parameters are part of maxStackHeight
temp = createEOF1Code(["60FF" + "E30001" + "00", "50" + "E4"], [1, 1])
temp.code[1].stackInputs = 1
temp.code[1].stackOutputs = 0
eof1GoodList.push([temp, "EOFV0010", "ok."])

// We can't dig below our parameters
// with POP
temp = createEOF1Code(["5F80E3000100", "505050" + "E4"], [0, 0])
temp.code[0].maxStack = 2
temp.code[1].stackInputs = 2
temp.code[1].stackOutputs = 0
 eof1BadList.push([temp, "EOF1I0014", "EOF_StackUnderflow"])

// DUP
temp = createEOF1Code(["5F80E3000100", "81505050" + "E4"], [0, 3])
temp.code[0].maxStack = 2
temp.code[1].stackInputs = 2
temp.code[1].stackOutputs = 0
eof1GoodList.push([temp, "EOF1I0014", "ok."])

temp = createEOF1Code(["5F80E3000100", "82505050" + "E4"], [0, 3])
temp.code[0].maxStack = 2
temp.code[1].stackInputs = 2
temp.code[1].stackOutputs = 0
 eof1BadList.push([temp, "EOF1I0014", "EOF_StackUnderflow"])


// SWAP
temp = createEOF1Code(["5F80E3000100", "905050" + "E4"], [0, 2])
temp.code[0].maxStack = 2
temp.code[1].stackInputs = 2
temp.code[1].stackOutputs = 0
eof1GoodList.push([temp, "", "ok."])

temp = createEOF1Code(["5F80E3000100", "915050" + "E4"], [0, 2])
temp.code[0].maxStack = 2
temp.code[1].stackInputs = 2
temp.code[1].stackOutputs = 0
 eof1BadList.push([temp, "EOF1I0014", "EOF_StackUnderflow"])



// Removed opcodes
// Try an old style jump, see it fails
 eof1BadList.push([createEOF1Code(["E3000100", "5B600056E4"], [0, 1]), "EOF1I0015", "EOF_UndefinedInstruction"])

// Old style conditional jump
 eof1BadList.push([createEOF1Code(["E3000100", "5B6001600057E4"], [0, 1]), "EOF1I0015", "EOF_UndefinedInstruction"])


// Suicidal code
 eof1BadList.push([createEOF1Code(["60016002FF00"], [2, 0]), "EOF1I0015", "EOF_UndefinedInstruction"])


// CALLCODE
 eof1BadList.push([createEOF1Code(["6001600260036004600560066007F200"], [7, 0]), "EOF1I0015", "EOF_UndefinedInstruction"])



// Jumps
// New style jump
eof1GoodList.push([createEOF1Code(["E50001", "E0000000"], [0, 0]), "EOF1V0008", "ok."])

// New style jump that actually does something
eof1GoodList.push([createEOF1Code(["E3000100", "6001E10001E4E0FFFC"], [0, 1]), "EOF1V0008", "ok."])

eof1GoodList.push([createEOF1Code(["E3000100", "E00000E00000E4"], [0, 0]), "EOF1V0008", "ok."])


// New style jump to hyperspace
 eof1BadList.push([createEOF1Code(["E3000100", "E00010E4"], [0, 0]), "EOF1I0016", "EOF_InvalidJumpDestination"])
 eof1BadList.push([createEOF1Code(["E3000100", "E0FF00E4"], [0, 0]), "EOF1I0016", "EOF_InvalidJumpDestination"])

// Jump to just before or after the code (bug that got solved)
 eof1BadList.push([createEOF1Code(["E30001E50002", "E00001E4", "00"], [0, 0, 0]), "EOF1I0016", "EOF_InvalidJumpDestination"])
 eof1BadList.push([createEOF1Code(["E30001E50002", "E0FFFCE4", "00"], [0, 0, 0]), "EOF1I0016", "EOF_InvalidJumpDestination"])


temp = createEOF1Code(["E30001505000", "600160026003E4"], [0, 3])
temp.code[0].maxStack = 2
temp.code[1].stackInputs = 0
temp.code[1].stackOutputs = 2
 eof1BadList.push([temp, "EOF1I0013", "EOF_InvalidNumberOfOutputs"])

// New style jump into the middle of an instruction
 eof1BadList.push([createEOF1Code(["E3000100", "E0000160FF60FFE4"], [0, 1]), "EOF1I0019", "EOF_InvalidJumpDestination"])


// New style conditional jump
eof1GoodList.push([createEOF1Code(["E50001", "6000" + "E10000" + "30" + "00"], [0, 1]),
		"EOF1V0011", "ok."])

// New style conditional jump that actually does something
eof1GoodList.push([createEOF1Code(["E50001", "6000" + "E10002" + "3050" + "3000"], [0, 1]),
		"EOF1V0011", "ok."])

// New style conditional jump into the middle of an instruction
 eof1BadList.push([createEOF1Code(["E3000100", "6001E1000160FF50E4"], [0, 1]), "EOF1I0019", "EOF_InvalidJumpDestination"])

// Two different pathes to the same opcode, with the same stack height
eof1GoodList.push([createEOF1Code(["E50001", "6000" + "E10002" + "3050" + "3000"], [0, 1]),
		"EOF1V0011", "ok."])


// Two different pathes to the same opcode, with different stack heights
 eof1GoodList.push([createEOF1Code(["E50001", "6000" + "E10001" + "30" + "3000"], [0, 2]),
		"EOF1I0020", "ok."])


// section output affects maxStackHeight on the caller
temp = createEOF1Code(["E30001" + "50" + "00", "60FF" + "E4"], [1, 1])
temp.code[1].stackInputs = 0
temp.code[1].stackOutputs = 1
eof1GoodList.push([temp, "EOFV0012", "ok."])


// RJUMPV, use jump table
eof1GoodList.push(
  [
    createEOF1Code
      ([
        "6001" +   // PUSH1 1
        "E2"   +   // RJUMPV, use jump table
        "04"   +   // Jump table max index
        "0000" +   // Relative address, case 0
        "0000" +   // Relative address, case 1
        "0000" +   // Relative address, case 2
        "0000" +   // Relative address, case 3
        "0000" +   // Relative address, case 4
        "00"       // STOP
       ], [1]
      ), "EOF1V0013", "ok."])


// RJUMPV that actually does something
eof1GoodList.push(
  [
    createEOF1Code
      ([
        "6001" +   // PUSH1 1
        "E2"   +   // RJUMPV, use jump table
        "04"   +   // Jump table max index
        "0000" +   // Relative address, case 0
        "0001" +   // Relative address, case 1
        "0001" +   // Relative address, case 2
        "0000" +   // Relative address, case 3
        "0001" +   // Relative address, case 4
        "00" +     // STOP
        "305000"   // ADDRESS, POP, STOP
       ], [1]
      ), "EOF1V0013", "ok."])



// RJUMPV into the middle of an opcode
 eof1BadList.push(
  [
    createEOF1Code
      ([
        "6001" +   // PUSH1 1
        "E2"   +   // RJUMPV, use jump table
        "04"   +   // Jump table max index
        "0000" +   // Relative address, case 0
        "0001" +   // Relative address, case 1
        "0001" +   // Relative address, case 2
        "0002" +   // Relative address, case 3
        "0001" +   // Relative address, case 4
        "00" +     // STOP
        "60FF5000" // PUSH1, POP, STOP
       ], [1]
      ), "EOF1I0019", "EOF_InvalidJumpDestination"])


// RJUMPV into its own middle
 eof1BadList.push(
  [
    createEOF1Code
      ([
        "6001" +   // PUSH1 1
        "E2"   +   // RJUMPV, use jump table
        "04"   +   // Jump table max index
        "0000" +   // Relative address, case 0
        "0001" +   // Relative address, case 1
        "0001" +   // Relative address, case 2
        "FFFE" +   // Relative address, case 3
        "0001" +   // Relative address, case 4
        "00" +     // STOP
        "60FF5000" // PUSH1, POP, STOP
       ], [1]
      ), "EOF1I0019", "EOF_InvalidJumpDestination"])


// Unreachable code fails

// Uncoditional jump
eof1GoodList.push([createEOF1Code(["E0000000"], [0]), "", "ok."])
 eof1BadList.push([createEOF1Code(["E000010000"], [0]), "EOF1I0023", "EOF_UnreachableCode"])

// Conditional jump
eof1GoodList.push([createEOF1Code(["6001E1000100305000"], [1]), "", "ok."])
 eof1BadList.push([createEOF1Code(["6001E100020000305000"], [1]), "EOF1I0023", "EOF_UnreachableCode"])

// Jump table
eof1GoodList.push([createEOF1Code(["6001" +
                                   "E201" +
                                     "0002" +
                                     "0004" +
                                   "3050" +
                                   "3050" +
                                   "3050" +
                                   "00"], [1]), "", "ok."])


 eof1BadList.push([createEOF1Code(["6001" +
                                   "E201" +
                                     "0002" +
                                     "0004" +
                                   "3050" +
                                   "0000" +
                                   "3050" +
                                   "00"], [1]), "EOF1I0023", "EOF_UnreachableCode"])



// Stack underflow caused by a function call
temp = createEOF1Code(["3030E3000100", "5050" + "E4"], [2, 2])
temp.code[1].stackInputs = 2
temp.code[1].stackOutputs = 0
eof1GoodList.push([temp, "EOF1V0007", "ok."])

temp = createEOF1Code(["30E3000100", "5050" + "E4"], [1, 2])
temp.code[1].stackInputs = 2
temp.code[1].stackOutputs = 0
 eof1BadList.push([temp, "EOF1I0022", "EOF_StackUnderflow"])


// Sections that end with an opcode that isn't an approved terminator fail
eof1GoodList.push([createEOF1Code(["600060006000600000"], [4]), "EOF1V0014", "ok."])
eof1GoodList.push([createEOF1Code(["6000600060006000F3"], [4]), "EOF1V0014", "ok."])
eof1GoodList.push([createEOF1Code(["6000600060006000FD"], [4]), "EOF1V0014", "ok."])
eof1GoodList.push([createEOF1Code(["6000600060006000FE"], [4]), "EOF1V0014", "ok."])
 eof1BadList.push([createEOF1Code(["6000600060006000"], [4]), "EOF1I0024", "EOF_InvalidCodeTermination"])
 eof1BadList.push([createEOF1Code(["600060006000600001"], [4]), "EOF1I0024", "EOF_InvalidCodeTermination"])
 eof1BadList.push([createEOF1Code(["600060006000600034"], [4]), "EOF1I0024", "EOF_InvalidCodeTermination"])
 eof1BadList.push([createEOF1Code(["600060006000600003"], [4]), "EOF1I0024", "EOF_InvalidCodeTermination"])
 eof1BadList.push([createEOF1Code(["600060006000600060006000A4"], [6]), "EOF1I0024", "EOF_InvalidCodeTermination"])
 eof1BadList.push([createEOF1Code(["3050E4"], [1]), "EOF1I0024", "EOF_InvalidNonReturningFlag"])



badContracts = eof1BadList.map(c => [encode(c[0]), c[1], c[2]]).
	map(c => [c[0], c[1], c[2]])

//console.log("GoodLIst: ", eof1GoodList)

goodContracts = eof1GoodList.map(c => [encode(c[0]), c[1], c[2]]).
	map(c => [c[0], c[1], c[2]])


const result = `
validInvalid:

# Try to create valid and invalid EOF1 encoded contracts
# See that only the valid ones turn into contracts
#
# Generated by src/Templates/EOF1/EOF1-generator.js

  _info:
    comment: |
      Ori Pomerantz  qbzzt1@gmail.com
      Implements EOF1I0001, EOF1I0002, EOF1I0003, EOF1I0004, EOF1I0005,
      EOF1I0006, EOF1I0007, EOF1I0008, EOF1I0009, EOF1I0010,
      EOF1I0011, EOF1I0012, EOF1I0013, EOF1I0014, EOF1I0015,
      EOF1I0016, EOF1I0017, EOF1I0018, EOF1I0019, EOF1I0020,
      EOF1I0021, EOF1I0022, EOF1I0023, EOF1I0024, EOF1I0025,
      EOF1V0001, EOF1V0002, EOF1V0003, EOF1V0004, EOF1V0005,
      EOF1V0006, EOF1V0007, EOF1V0008, EOF1V0009, EOF1V0010,
      EOF1V0011, EOF1V0012, EOF1V0013, EOF1V0014, EOF1V0015

  forks:
    - ">=Prague"

  vectors:
${badContracts.map((code, i) => `
      ${code[1] == "" ? `# Data ${i}` : `# Data ${i} implements ${code[1]}`}
      - data: |
         :label ${code[1]}  :raw 0x${code[0]}
        ${code[2] == "ok." ? `# ">=Prague" : "ok."` : `expectException:`}
        ${code[2] == "ok." ? `` : `  ">=Prague" : "${code[2]}"`}
`).reduce((a,b) => a+b)}

${goodContracts.map((code, i) => `
      ${code[1] == "" ? `# Data ${i+badContracts.length}` : `# Data ${i+badContracts.length} implements ${code[1]}`}
      - data: |
         :label ${code[1]} :raw 0x${code[0]}
        ${code[2] == "ok." ? `# ">=Prague" : "ok."` : `expectException:`}
        ${code[2] == "ok." ? `` : `  ">=Prague" : "${code[2]}"`}
`).reduce((a,b) => a+b)}

`


console.log(result)
