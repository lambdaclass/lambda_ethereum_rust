#! /usr/bin/bash

# Create template tests that run the same code in difference places
# (just run it, run it in a called account, etc. - the full list is
# in templateGen.js


./createBadOpcodeTest.sh 0C
./createBadOpcodeTest.sh 0D
./createBadOpcodeTest.sh 0E
./createBadOpcodeTest.sh 0F
./createBadOpcodeTest.sh 1E
./createBadOpcodeTest.sh 1F
./createBadOpcodeTest.sh 21
./createBadOpcodeTest.sh 22
./createBadOpcodeTest.sh 23
./createBadOpcodeTest.sh 24
./createBadOpcodeTest.sh 25
./createBadOpcodeTest.sh 26
./createBadOpcodeTest.sh 27
./createBadOpcodeTest.sh 28
./createBadOpcodeTest.sh 29
./createBadOpcodeTest.sh 2A
./createBadOpcodeTest.sh 2B
./createBadOpcodeTest.sh 2C
./createBadOpcodeTest.sh 2D
./createBadOpcodeTest.sh 2E
./createBadOpcodeTest.sh 2F
./createBadOpcodeTest.sh 49 London,Paris,Shanghai   # BLOBHASH
./createBadOpcodeTest.sh 4A London,Paris,Shanghai   # BEACON_ROOT
./createBadOpcodeTest.sh 4B
./createBadOpcodeTest.sh 4C
./createBadOpcodeTest.sh 4D
./createBadOpcodeTest.sh 4E
./createBadOpcodeTest.sh 4F
./createBadOpcodeTest.sh 5C London,Paris,Shanghai   # TLOAD
./createBadOpcodeTest.sh 5D London,Paris,Shanghai   # TSTORE
./createBadOpcodeTest.sh 5E London,Paris,Shanghai   # MCOPY
./createBadOpcodeTest.sh 5F London,Paris            # PUSH0
./createBadOpcodeTest.sh A5
./createBadOpcodeTest.sh A6
./createBadOpcodeTest.sh A7
./createBadOpcodeTest.sh A8
./createBadOpcodeTest.sh A9
./createBadOpcodeTest.sh AA
./createBadOpcodeTest.sh AB
./createBadOpcodeTest.sh AC
./createBadOpcodeTest.sh AD
./createBadOpcodeTest.sh AE
./createBadOpcodeTest.sh AF

./createBadOpcodeTest.sh B0
./createBadOpcodeTest.sh B1
./createBadOpcodeTest.sh B2
./createBadOpcodeTest.sh B3
./createBadOpcodeTest.sh B4
./createBadOpcodeTest.sh B5
./createBadOpcodeTest.sh B6
./createBadOpcodeTest.sh B7
./createBadOpcodeTest.sh B8
./createBadOpcodeTest.sh B9
./createBadOpcodeTest.sh BA
./createBadOpcodeTest.sh BB
./createBadOpcodeTest.sh BC
./createBadOpcodeTest.sh BD
./createBadOpcodeTest.sh BE
./createBadOpcodeTest.sh BF

./createBadOpcodeTest.sh C0
./createBadOpcodeTest.sh C1
./createBadOpcodeTest.sh C2
./createBadOpcodeTest.sh C3
./createBadOpcodeTest.sh C4
./createBadOpcodeTest.sh C5
./createBadOpcodeTest.sh C6
./createBadOpcodeTest.sh C7
./createBadOpcodeTest.sh C8
./createBadOpcodeTest.sh C9
./createBadOpcodeTest.sh CA
./createBadOpcodeTest.sh CB
./createBadOpcodeTest.sh CC
./createBadOpcodeTest.sh CD
./createBadOpcodeTest.sh CE
./createBadOpcodeTest.sh CF

./createBadOpcodeTest.sh D0
./createBadOpcodeTest.sh D1
./createBadOpcodeTest.sh D2
./createBadOpcodeTest.sh D3
./createBadOpcodeTest.sh D4
./createBadOpcodeTest.sh D5
./createBadOpcodeTest.sh D6
./createBadOpcodeTest.sh D7
./createBadOpcodeTest.sh D8
./createBadOpcodeTest.sh D9
./createBadOpcodeTest.sh DA
./createBadOpcodeTest.sh DB
./createBadOpcodeTest.sh DC
./createBadOpcodeTest.sh DD
./createBadOpcodeTest.sh DE
./createBadOpcodeTest.sh DF

./createBadOpcodeTest.sh E0 London,Paris,Shanghai,Cancun    # RJUMP
./createBadOpcodeTest.sh E1 London,Paris,Shanghai,Cancun    # RJUMI
./createBadOpcodeTest.sh E2 London,Paris,Shanghai,Cancun    # RJUMV
./createBadOpcodeTest.sh E3 London,Paris,Shanghai,Cancun    # CALLF
./createBadOpcodeTest.sh E4 London,Paris,Shanghai,Cancun    # RETF
./createBadOpcodeTest.sh E5 London,Paris,Shanghai,Cancun    # JUMPF
./createBadOpcodeTest.sh E6 London,Paris,Shanghai,Cancun    # DUPN
./createBadOpcodeTest.sh E7 London,Paris,Shanghai,Cancun    # SWAPN
./createBadOpcodeTest.sh E8 London,Paris,Shanghai,Cancun    # DATALOAD
./createBadOpcodeTest.sh E9 London,Paris,Shanghai,Cancun    # DATALOADN
./createBadOpcodeTest.sh EA London,Paris,Shanghai,Cancun    # DATASIZE
./createBadOpcodeTest.sh EB London,Paris,Shanghai,Cancun    # DATACOPY
./createBadOpcodeTest.sh EC London,Paris,Shanghai,Cancun    # CREATE3
./createBadOpcodeTest.sh ED London,Paris,Shanghai,Cancun    # RETURNCONTRACT
./createBadOpcodeTest.sh EE
# Special case, because it's also the magic value for EOF1
# ./createBadOpcodeTest.sh EF

./createBadOpcodeTest.sh F6 London,Paris,Shanghai,Cancun    # PAY
./createBadOpcodeTest.sh F7 London,Paris,Shanghai,Cancun    # CREATE4
./createBadOpcodeTest.sh F8 London,Paris,Shanghai,Cancun    # CALL2
./createBadOpcodeTest.sh F9 London,Paris,Shanghai,Cancun    # DELEGATECALL2
./createBadOpcodeTest.sh FB London,Paris,Shanghai,Cancun    # STATICCALL2
./createBadOpcodeTest.sh FC
./createBadOpcodeTest.sh FE
