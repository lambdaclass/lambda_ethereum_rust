# List of test vectors ids that can be found in tests
## Withdrawals tests EIP-4895

- ID: **BSHA0001** check that Shanghai block rejected before transition
- ID: **BSHA0002** check that Paris block rejected after transition
- ID: **BSHA0003** check that Block without withdrawals root header field but with withdrawals in the body gets rejected after transition
- ID: **BSHA0004** check that if withdrawals root does not match the withdrawals body block rejected
- ID: **BSHA0005** check that Shanghai block without withdrawals rlp body is rejected as incomplete
- ID: **BSHA0006** check if withdrawals rlp body is not presented as List

### Withdrawals elements bounds
- ID: **BSHB0001** withdrawals index is 2^64-1
- ID: **BSHB0002** withdrawals index is 2^64
- ID: **BSHB0003** withdrawals index is bigint
- ID: **BSHB0004** withdrawals validatorIndex 2^64-1
- ID: **BSHB0005** withdrawals validatorIndex 2^64
- ID: **BSHB0006** withdrawals validatorIndex bigint
- ID: **BSHB0007** withdrawals address is 0x0000..00
- ID: **BSHB0008** withdrawals address is less than 20 bytes
- ID: **BSHB0009** withdrawals address is more than 20 bytes
- ID: **BSHB0010** withdrawals amount 0 is fine
- ID: **BSHB0011** withdrawals amount more 2^256 is bad
- ID: **BSHB0012** withdrawals amount more 2^64 is ok
- ID: **BSHB0013** withdrawals body has more than 4 elements
- ID: **BSHB0014** withdrawals body has less than 4 elements
- ID: **BSHB0015** two withdrawals record with the same index  (WARNING: ALLOWED)
- ID: **BSHB0016** different validators withdraw to the same address
- ID: **BSHB0017** withdrawal amount is 0 (WARNING: touch empty account)
- ID: **BSHB0018** withdrawal amount is 0, plus transaction
- ID: **BSHB0019** withdrawal happens to account that suicide in the same block
- ID: **BSHB0020** withdrawal happens to account that receive funds in the same block
- ID: **BSHB0021** check the balance instruction on withdrawal received account before block fin
- ID: **BSHB0022** withdrawal receiving account trys to send value before block fin
- ID: **BSHB0023,b** account suicide, then receive funds, then receive withdrawal
- ID: **BSHB0024,b** account created, then reverted, then receive withdrawals
- ID: **BSHB0025** check staticcall is fine with withdrawals
- ID: **BSHB0026** check operations gasprice with withdrawals context


## EOF1 contracts

### Invalid examples

- ID: **EOF1I0001** check that EOF1 with a bad magic number fails
- ID: **EOF1I0002** check that EOF1 with a bad version number fails
- ID: **EOF1I0003** check that EOF1 with a bad section order fails
- ID: **EOF1I0004** check that EOF1 missing a section fails
- ID: **EOF1I0005** check that EOF1 with a bad end of sections number fails
- ID: **EOF1I0006** check that EOF1 with too many or too few bytes fails
- ID: **EOF1I0007** check that EOF1 with a malformed code section fails
- ID: **EOF1I0008** check that EOF1 with an illegal opcode fails
- ID: **EOF1I0009** check that EOF1 with the wrong maxStackDepth fails
- ID: **EOF1I0010** check that return values are not allowed on section 0
- ID: **EOF1I0011** check that function calls to code sections that don't exist fail
- ID: **EOF1I0012** check that code sections that cause stack underflow fail
- ID: **EOF1I0013** check that we can't return more values than we declare
- ID: **EOF1I0014** check that code that looks deeper in the stack than the parameters fails
- ID: **EOF1I0015** check that code that uses removed opcodes fails
- ID: **EOF1I0016** check that code that uses new relative jumps to outside the section fails
- ID: **EOF1I0017** check that parameters are not allowed on section 0
- ID: **EOF1I0018** inconsistent number of code sections (between types and code)
- ID: **EOF1I0019** check that jumps into the middle on an opcode are not allowed
- ID: **EOF1I0020** check that you can't get to the same opcode with two different stack heights
- ID: **EOF1I0021** empty jump table
- ID: **EOF1I0022** stack underflow caused by a function call
- ID: **EOF1I0023** sections with unreachable code fail
- ID: **EOF1I0024** sections that end with a non-terminator opcode fail
- ID: **EOF1I0025** data stack height of 1024 is invalid
- ID: **EOF1I3540_0001** No magic - Data index: 4
- ID: **EOF1I3540_0002-0004** Invalid magic
- ID: **EOF1I3540_0005** No version
- ID: **EOF1I3540_0006-0008** Invalid version
- ID: **EOF1I3540_0009** No header
- ID: **EOF1I3540_0010** No type section size
- ID: **EOF1I3540_0011** Type section size incomplete
- ID: **EOF1I3540_0012** Empty code section with no
- ID: **EOF1I3540_0013** No total of code sections
- ID: **EOF1I3540_0014** Total of code sections incomplete
- ID: **EOF1I3540_0015** No code section size
- ID: **EOF1I3540_0016** Code section size incomplete
- ID: **EOF1I3540_0017** No data section after code section size
- ID: **EOF1I3540_0018** No data size
- ID: **EOF1I3540_0019** Data size incomplete
- ID: **EOF1I3540_0020** No section terminator after data section size
- ID: **EOF1I3540_0021** No type section contents
- ID: **EOF1I3540_0022-0024** Type section contents (no outputs and max stack)
- ID: **EOF1I3540_0025** No code section contents
- ID: **EOF1I3540_0026** Code section contents incomplete
- ID: **EOF1I3540_0027** Trailing bytes after code section
- ID: **EOF1I3540_0028** Empty code section
- ID: **EOF1I3540_0029** Empty code section with no
- ID: **EOF1I3540_0030** Code section preceding type section
- ID: **EOF1I3540_0031** Data section preceding type section
- ID: **EOF1I3540_0032** Data section preceding code section
- ID: **EOF1I3540_0033** Data section without code section
- ID: **EOF1I3540_0034** No data section
- ID: **EOF1I3540_0035** Trailing bytes after data section
- ID: **EOF1I3540_0036** Multiple data sections
- ID: **EOF1I3540_0037** Multiple code and data sections
- ID: **EOF1I3540_0038-0040** Unknown section IDs (at the beginning)
- ID: **EOF1I3540_0041-0043** Unknown section IDs (after types section)
- ID: **EOF1I3540_0044-0046** Unknown section IDs (after code section)
- ID: **EOF1I3540_0047-0049** Unknown section IDs (after data section)
- ID: **EOFI3670_0140-0235** Code containing undefined instruction
- ID: **EOFI3670_0236-0298** Truncated PUSH*
- ID: **EOFI3670_0299** Containing undefined instruction (0xfb) after STOP
- ID: **EOF1I4200_0001** EOF code containing truncated RJUMP
- ID: **EOF1I4200_0002** EOF code containing truncated RJUMP
- ID: **EOF1I4200_0003-0007** EOF code containing RJUMP with target outside code bounds
- ID: **EOF1I4200_0008-0013** EOF code containing RJUMP with target instruction immediate
- ID: **EOF1I4200_0014-0015** EOF code containing truncated RJUMPI
- ID: **EOF1I4200_0016-0020** EOF code containing RJUMPI with target outside code bounds
- ID: **EOF1I4200_0021-0026** EOF code containing RJUMPI with target instruction immediate
- ID: **EOF1I4200_0027** EOF code containing RJUMPV with max_index 0 but no immediates
- ID: **EOF1I4200_0028-0030** EOF code containing truncated RJUMPV
- ID: **EOF1I4200_0031-0035** EOF code containing RJUMPV with target outside code bounds
- ID: **EOF1I4200_0036-0041** EOF code containing RJUMPV with target instruction immediate
- ID: **EOF1I4750_0001** EOF code missing mandatory type section
- ID: **EOF1I4750_0002** EOF code containing multiple type headers
- ID: **EOF1I4750_0003-0005** EOF code containing type section size (Size 1)
- ID: **EOF1I4750_0006-0008** EOF code containing invalid section type
- ID: **EOF1I4750_0009** EOF code containing too many code sections
- ID: **EOF1I4750_0010** EOF code containing CALLF to a non existing code section
- ID: **EOF1I4750_0011** EOF code containing truncated CALLF
- ID: **EOF1I4750_0018** EOF code containing deprecated instruction (JUMP, PC, SELFDESTRUCT, CALLCODE, CREATE, CREATE2)
- ID: **EOF1I4750_0019** EOF code containing call to functions without required stack specified in type section
- ID: **EOF1I4750_0020** EOF code containing call to functions without required stack NOT specified in type section
- ID: **EOF1I4750_0021** EOF code containing function trying to return more items than specified in type section
- ID: **EOF1I4750_0022** EOF code containing function exceeding max stack items
- ID: **EOF1I4750_0023** EOF code containing function which max stack height causes to exceed max stack items (stack overflow)
- ID: **EOF1I4750_0027** EOF code containing RETF as terminating instruction in first code section
- ID: **EOF1I5450_0001-0002** Pushing loop
- ID: **EOF1I5450_0003-0087** Stack underflow
- ID: **EOF1I5450_0088-89** Calling function without enough stack items
- ID: **EOF1I5450_0090** Stack Overflow: Function pushing more than 1024 items to the stack
- ID: **EOF1I5450_0091** Stack Overflow: Function 1 when called by Function 0 pushes more than 1024 items to the stack
- ID: **EOF1I5450_0092-0094** Function ending with no-terminating instruction
- ID: **EOF1I5450_0095** Function containing unreachable code after RETURN
- ID: **EOF1I5450_0096** Function containing unreachable code after REVERT
- ID: **EOF1I5450_0097** Unreachable code after RJUMP
- ID: **EOF1I5450_0098** Unreachable code after infinite loop



### Valid examples

- ID: **EOF1V0001** check that simple valid EOF1 deploys
- ID: **EOF1V0002** check that valid EOF1 with two code sections deploys
- ID: **EOF1V0003** check that valid EOF1 with four code sections deploys
- ID: **EOF1V0004** check that valid EOF1 can include 0xFE, the designated "invalid opcode"
- ID: **EOF1V0005** check that EOF1 with the right maxStackDepth deploys
- ID: **EOF1V0006** check that return values are allowed on code sections that aren't zero
- ID: **EOF1V0007** check that function calls to code sections that exist are allowed
- ID: **EOF1V0008** check that code that uses a new style relative jump (5C) succeeds
- ID: **EOF1V0009** check that parameters are allowed on code sections that aren't zero
- ID: **EOF1V0010** parameters are part of the max stack height
- ID: **EOF1V0011** check that code that uses a new style conditional jump (5D) succeeds
- ID: **EOF1V0012** return values on code sections affect maxStackHeight of the caller
- ID: **EOF1V0013** jump tables work
- ID: **EOF1V0014** sections that end with a legit terminating opcode are OK
- ID: **EOF1V0015** data stack height of 1023 is valid
- ID: **EOF1V0016** check that data section size can be less than the declared size
- ID: **EOF1V3540_0001** Deployed code without data section
- ID: **EOF1V3540_0002** Deployed code with data section
- ID: **EOF1V3540_0003** No data section contents (valid according to relaxed stack validation)
- ID: **EOF1V3540_0004** Data section contents incomplete (valid according to relaxed stack validation)
- ID: **EOFV3670_0001-0139** Code containing each valid opcode
- ID: **EOF1V4200_0001** EOF code containing RJUMP (Positive, Negative)
- ID: **EOF1V4200_0002** EOF code containing RJUMP (Zero)
- ID: **EOF1V4200_0003** EOF with RJUMP containing the maximum offset (32767)
- ID: **EOF1V4200_0004** EOF code containing RJUMPI (Positive)
- ID: **EOF1V4200_0005** EOF code containing RJUMPI (Negative)
- ID: **EOF1V4200_0006** EOF code containing RJUMPI (Zero)
- ID: **EOF1V4200_0007** EOF with RJUMPI containing the maximum offset (32767)
- ID: **EOF1V4200_0008** EOF with RJUMPV table size 1 (Positive)
- ID: **EOF1V4200_0009** EOF with RJUMPV table size 1 (Negative)
- ID: **EOF1V4200_0010** EOF with RJUMPV table size 1 (Zero)
- ID: **EOF1V4200_0011** EOF with RJUMPV table size 3
- ID: **EOF1V4200_0012** EOF with RJUMPV table size 256 (Target 0)
- ID: **EOF1V4200_0013** EOF with RJUMPV table size 256 (Target 100)
- ID: **EOF1V4200_0014** EOF with RJUMPV table size 256 (Target 254)
- ID: **EOF1V4200_0015** EOF with RJUMPV table size 256 (Target 256)
- ID: **EOF1V4200_0016** EOF with RJUMPV containing the maximum offset (32767)
- ID: **EOF1V4750_0001** EOF code containing single type section
- ID: **EOF1V4750_0002** EOF code containing single type section and data section
- ID: **EOF1V4750_0003** EOF code containing multiple type/code sections
- ID: **EOF1V4750_0004** EOF code containing multiple type/code sections and data section
- ID: **EOF1V4750_0005** EOF code containing multiple type/code sections, no void I/O types
- ID: **EOF1V4750_0006** EOF code containing multiple type/code sections, no void I/O types, containing data section
- ID: **EOF1V4750_0007** EOF code containing the maximum number of code sections
- ID: **EOF1V4750_0008** EOF code containing the maximum number of code sections and data section
- ID: **EOF1V5450_0001** Code with branches having the same stack height
- ID: **EOF1V5450_0002** Jump table
- ID: **EOF1V5450_0003** Infinite loop
- ID: **EOF1V5450_0004** Infinite loop using RJUMPV
- ID: **EOF1V5450_0005** CALLF branches with the same total of outputs
- ID: **EOF1V5450_0006** CALLF inputs
- ID: **EOF1V5450_0007-0091** Validate input for all opcodes requiring stack item(s)
- ID: **EOF1V5450_0092** Containing terminating opcode RETURN at the end
- ID: **EOF1V5450_0093** Containing terminating opcode REVERT at the end
- ID: **EOF1V5450_0094** Loop ending with unconditional RJUMP (a)
- ID: **EOF1V5450_0095** Loop ending with unconditional RJUMP (b)
- ID: **EOF1V5450_0096** Functions ending with RETF
- ID: **EOF1V5450_0097** Stack is not required to be empty on terminating instruction RETURN
- ID: **EOF1V5450_0098** Stack is not required to be empty on terminating instruction REVERT
- ID: **EOF1V5450_0099** RETF returning maximum number of outputs (127)
- ID: **EOF1V5450_0100** Calling function with enough stack items: Function 1 calls Function 2 with enough parameters
- ID: **EOF1V5450_0101** Stack height mismatch for different paths valid according to relaxed stack validation
- ID: **EOF1V5450_0102** Stack height mismatch for different paths valid according to relaxed stack validation
- ID: **EOF1V5450_0103** Calls returning different number of outputs valid according to relaxed stack validation
- ID: **EOF1V5450_0104** Jump table with different stack heights valid according to relaxed stack validation

### Contract creation tests

- ID: **EOF1C0001** legacy contract > CREATE > legacy init code > legacy deploy code
- ID: **EOF1C0002** legacy contract > CREATE > legacy init code > eof1 deploy code
- ID: **EOF1C0003** legacy contract > CREATE > eof1 init code > legacy deploy code (fail)
- ID: **EOF1C0004** legacy contract > CREATE > eof1 init code > eof1 deploy code
- ID: **EOF1C0005** legacy contract > CREATE2 > legacy init code > legacy deploy code
- ID: **EOF1C0006** legacy contract > CREATE2 > legacy init code > eof1 deploy code
- ID: **EOF1C0007** legacy contract > CREATE2 > eof1 init code > legacy deploy code (fail)
- ID: **EOF1C0008** legacy contract > CREATE2 > eof1 init code > eof1 deploy code
- ID: **EOF1C0009** eof1 contract > CREATE > legacy init code > legacy deploy code (fail)
- ID: **EOF1C0010** eof1 contract > CREATE > legacy init code > eof1 deploy code (fail)
- ID: **EOF1C0011** eof1 contract > CREATE > eof1 init code > legacy deploy code (fail)
- ID: **EOF1C0012** eof1 contract > CREATE > eof1 init code > eof1 deploy code
- ID: **EOF1C0013** eof1 contract > CREATE2 > legacy init code > legacy deploy code (fail)
- ID: **EOF1C0014** eof1 contract > CREATE2 > legacy init code > eof1 deploy code (fail)
- ID: **EOF1C0015** eof1 contract > CREATE2 > eof1 init code > legacy deploy code (fail)
- ID: **EOF1C0016** eof1 contract > CREATE2 > eof1 init code > eof1 deploy code
- ID: **EOF1C0017** legacy contact > CREATE[2] > legacy init code > nonsense
- ID: **EOF1C0018** legacy contact > CREATE[2] > eof1 init code > nonsense (fail)
- ID: **EOF1C0019** legacy contact > CREATE[2] > nonsense init code (fail)


## Warm COINBASE EIP-3651 

- ID: **WRMCB0001** check EXTCODESIZE gas cost on coinbase
- ID: **WRMCB0002** check EXTCODECOPY gas cost on coinbase
- ID: **WRMCB0003** check EXTCODEHASH gas cost on coinbase
- ID: **WRMCB0004** check BALANCE gas cost on coinbase
- ID: **WRMCB0005** check CALL gas cost on coinbase
- ID: **WRMCB0006** check CALLCODE gas cost on coinbase
- ID: **WRMCB0007** check DELEGATECALL gas cost on coinbase
- ID: **WRMCB0008** check STATICCALL gas cost on coinbase
- ID: **WRMCB0009** check OOG CALL on coinbase
- ID: **WRMCB0010** check OOG CALLCODE on coinbase
- ID: **WRMCB0011** check OOG DELEGATECALL on coinbase
- ID: **WRMCB0012** check OOG STATICCALL on coinbase


## SELFDESTRUCT and how it affects balances
- ID: **SUC000** contractA is called to perform suicide to dest1 (3 must immidiately(check that it is immidiately and not by the end of tx) go to dest1)
- ID: **SUC001** does not suicide, just receive more funds via call. check the balance of contractA (must be 0, the second transfer funds erased)
- ID: **SUC002** does another suicide to dest1 (so dest1 now have 3 + x)
- ID: **SUC003** does another suicide to dest2 (so both dest1 and dest2 has some balance)
- ID: **SUC004** tries to make value transfer but fails because initial suicide 0ed the balance (answering this (check that it is immidiately and not by the end of tx))
- ID: **SUC005** receives more funds and try to transder it (same as SUC004, but this time work because it received funds via 2nd call)
- ID: **SUC006** receive funds post-transaction, create a mostly empty address
- ID: **SUC007.0** Same txn, same addr, different frames. Frame1 CALLs frame2, frame2 SELFDESTRUCTs into an address, and then frame1 SELFDESTRUCTs into another address. Balance ends up where frame2 sent it.
- ID: **SUC007.1** Same txn, same addr, different frames. Frame1 CALLCODEs frame2, frame2 SELFDESTRUCTs into an address, and then frame1 SELFDESTRUCTs into another address. Balance ends up where frame2 sent it.
- ID: **SUC007.2** Same txn, same addr, different frames. Frame1 DELEGATECALLs frame2, frame2 SELFDESTRUCTs into an address, and then frame1 SELFDESTRUCTs into another address. Balance ends up where frame2 sent it.
- ID: **SUC007.3** Same txn, same addr, different frames. Frame1 STATICCALLs frame2, frame2 SELFDESTRUCTs into an address, and then frame1 SELFDESTRUCTs into another address. Balance ends up where frame1 sent it (static frames can't SELFDESTRUCT).
- ID: **SUC008.0** Same txn, same addr, different frames. Frame1 CALLs frame2, frame2 SELFDESTRUCTs into contract's own ADDR, and then frame1 SELFDESTRUCTs into another address. Balance disappears.
- ID: **SUC008.1** Same txn, same addr, different frames. Frame1 CALLCODEs frame2, frame2 SELFDESTRUCTs into contract's own ADDR, and then frame1 SELFDESTRUCTs into another address. Balance disappears.
- ID: **SUC008.2** Same txn, same addr, different frames. Frame1 DELEGATECALLs frame2, frame2 SELFDESTRUCTs into contract's own ADDR, and then frame1 SELFDESTRUCTs into another address. Balance disappears.
- ID: **SUC008.3** Same txn, same addr, different frames. Frame1 STATICCALLs frame2, frame2 SELFDESTRUCTs into contract's own ADDR, and then frame1 SELFDESTRUCTs into the same address. Balance ends up where frame1 sent it (static frames can't SELFDESTRUCT).

## BLOBHASH EIP4844
- ID: **BLOB000** empty blobhash list transaction is rejected
- ID: **BLOB001** blobhash list element with wrong version byte transaction is rejected
- ID: **BLOB002** blobhash create transaction is rejected
- ID: **BLOB003** blobhash opcode return out of range
- ID: **BLOB004** same blobhash in the list
- ID: **BLOB005** blobhash opcode bounds
- ID: **BLOB006** see how many blobhash records are allowed per tx