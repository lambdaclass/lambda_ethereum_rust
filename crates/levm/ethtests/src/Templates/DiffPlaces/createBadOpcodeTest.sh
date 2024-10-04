#! /bin/bash

# Given a HEX opcode, create a template test for it
#
# YOU SHOULD NOT RUN THIS DIRECTLY. INSTEAD, USE createTemplateTests.sh

op=$1
stateTestDir=../../GeneralStateTestsFiller
testName=opc${op}DiffPlaces

# The default fork is "anything after London". However,
# some bad opcodes reformed and became good in Cancun
# (and some will become good later).
if [ -z "$2" ]
then
    fork=">=London"
else
    fork=$2
fi


# Create the appropriate YUL file
cat anyBadOpcode.yul | sed s/XX/$op/ > temp.yul


./templateGen.js $testName temp.yul 0x60A7 $fork \
     > $stateTestDir/stBadOpcode/${testName}Filler.yml

rm temp.yul
