#! /usr/bin/bash

# Usage:
# oneTest.sh <test type> <test path> <path for error files> <if value, don't use docker>
#
# This script is supposed to be executed by runTests.sh, not run directly by a user

suite=GeneralStateTests/stExample
test=random

testType=$1
testPath=$2
errPath=$3
nodocker=$4

echo Test type $testType

if [ $nodocker ]
then
   cmd="retesteth -t $suite -- "
else
   cmd="./dretesteth.sh -t $suite -- --datadir /tests/config "
fi


# Create a random test
./randomTest.js > $testPath/src/GeneralStateTestsFiller/stExample/${test}Filler.yml

cd ~

# Run the random test on Geth
if [ "$testType" == "block" ]
then
    $cmd --testpath $testPath --singletest $test \
         --all --fillchain --filltests
else
    $cmd --testpath $testPath --singletest $test \
         --all --filltests
fi

gethRetVal=$?
errFname=$testType-`dbus-uuidgen`

if [ $gethRetVal -gt 0 ]; then
  cat $testPath/src/GeneralStateTestsFiller/stExample/randomFiller.yml | \
        sed "s/random:/$errFname:/" > $errPath/${errFname}Filler.yml
  exit 1
fi


sleep 5   # Prevent race problems

# Run the same random test on Besu
# Run the random test on Geth
if [ "$testType" == "block" ]
then
    cmd=`echo $cmd | sed 's/-t /-t BC/'`
    $cmd --testpath $testPath --singletest $test \
         --all --clients besu
else
     $cmd --testpath $testPath --singletest $test \
          --all --clients besu
fi



besuRetVal=$?

if [ $besuRetVal -gt 0 ]; then
  cat $testPath/src/GeneralStateTestsFiller/stExample/randomFiller.yml | \
        sed "s/random:/$errFname:/" > $errPath/${errFname}Filler.yml
fi

