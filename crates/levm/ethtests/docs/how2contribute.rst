.. how2contribute:

###########################################
How to Contribute Tests
###########################################
`Ori Pomerantz <mailto://qbzzt1@gmail.com>`_

You've written a useful test and now you want to contribute it to 
`the repository <https://github.com/ethereum/tests>`_. This tutorial
teaches you how to do it.


Generalizing Tests
########################

Many of our tests started from the need to test a 
single scenario, but we figured out related scenarios that are also worth
testing. For example, `this test 
<https://github.com/ethereum/tests/blob/develop/src/GeneralStateTestsFiller/stBadOpcode/operationDiffGasFiller.yml>`_
started from `a request to make a CREATE2 opcode fail at a specific stage
<https://github.com/ethereum/tests/issues/909>`_.

However, there were two ways to generalize this:

* There are `multiple opcodes 
  <https://github.com/ethereum/tests/blob/develop/src/GeneralStateTestsFiller/stBadOpcode/operationDiffGasFiller.yml#L200-L216>`_ 
  that are probably implemented with a number of steps, each of which may have a gas cost.
* Why fail only at one step? If we `run the operation with different amounts of gas
  <https://github.com/ethereum/tests/blob/develop/src/GeneralStateTestsFiller/stBadOpcode/operationDiffGasFiller.yml#L182-L185>`_, 
  we can probably trigger failures at each step. 


While the state test transaction can only have one direct destination, we can provide
whatever data we want, and `that data can be used to calculate an address to call
<https://github.com/ethereum/tests/blob/develop/src/GeneralStateTestsFiller/stBadOpcode/operationDiffGasFiller.yml#L179>`_.
Different addresses can contain `contracts with different operations
<https://github.com/ethereum/tests/blob/develop/src/GeneralStateTestsFiller/stBadOpcode/operationDiffGasFiller.yml#L26-L161>`_.

The easiest way to run multiple tests in a state test is to use `different data values 
<state-transition-tutorial.html#multitest-files>`_. You can use the **:abi** encoding to
send multiple values. If you use **uint**, the first value will be available at 
**$4** (LLL) or **calldata(4)** (Yul), the second value at **$36** / **calldata(0x24)**, etc.




Files
############
The source/filler test file is written in either YML or JSON and located
under the **src** directory. This is the type of file explained 
`in the tutorials <state-transition-tutorial.html#the-source-code>`_.


In addition to the source file, your pull request needs to include the 
`generated/filled version(s) <internals-tutorial.html#compiled-tests>`_ 
of the test file. This version includes additional information, such as 
`merkle tree roots <https://en.wikipedia.org/wiki/Merkle_tree>`_ of the
current state, the compiled bytecode, etc.

.. note:: The directions below assume you are running **retesteth** through docker.
   `See here if you are not familiar with using **retesteth** that way.
   <retesteth-tutorial.html#retesteth-in-a-docker-container>`_



State Tests
=============
State tests have their source at 
**src/GeneralStateTestsFiller/<directory>/<test>Filler.<yml or json>**. 
Every state test is supposed to have two filled versions:

#. Filled state test, which is located in **GeneralStateTests/<directory>/<test>.json**.
   You use a command similar to this one to create this file:

   ::

      ./dretesteth.sh -t $suite -- --testpath $dir --singletest $test --filltests 

#. Blockchain state test, which is located in 
   **BlockchainTests/GeneralStateTests/<directory>/<test>.json**. 
   You use a command similar to this one to create this file:

   ::

      ./dretesteth.sh -t $suite -- --testpath $dir --singletest $test --fillchain





Blockchain Tests
================
State tests have their source at
**src/BlockchainTestsFiller/<directory>/<test>Filler.<yml or json>**.
These tests only have a single filled version, located in
**BlockchainTests/<directory>/<test>.json**.
You use a command similar to this one to create this file:

::

   ./dretesteth.sh -t $suite -- --testpath $dir --singletest $test --filltests




Conclusion
#################
At this point you should know enough to submit PRs with useful tests. Go write some
and amaze us.

