.. blockchain-tests-tutorial:

###########################################
Blockchain Tests
###########################################
`Ori Pomerantz <mailto://qbzzt1@gmail.com>`_

In this tutorial you learn how to use the skills you learned writing state tests to write
blockchain tests. These tests can include multiple blocks and each of those blocks can include
multiple transactions.

The Environment
===============
Before you start, make sure you create the retesteth tutorial and create the 
environment explained there. Also make sure you read and understand the state
transition tests tutorial.


Types of Blockchain Tests
=========================
If you go to **tests/src/BlockchainTestsFiller** you will see three different directories.

- **ValidBlocks** are tests that only have valid blocks, which the client should accept.

- **InvalidBlocks** are tests that should raise an exception because they 
  include invalid blocks.

- **TransitionTests** are tests that verify the transitions between different 
  versions of the Ethereum protocol (called `forks 
  <https://medium.com/mycrypto/the-history-of-ethereum-hard-forks-6a6dae76d56f>`_) 
  are handled correctly.
  
   

Valid Block Tests
=================
There is a valid block test in `tests/docs/tutorial_samples/05_simpleTxFiller.yml 
<https://github.com/ethereum/tests/blob/develop/docs/tutorial_samples/05_simpleTxFiller.yml>`_.
We copy it to **bcExample**.

::

   mkdir ~/tests/src/Blo*/Val*/bcExample*
   cp ~/tests/docs/tu*/05_* ~/tests/src/Blo*/Val*/bcExample*
   cd ~
   ./dretesteth.sh -t BlockchainTests/ValidBlocks/bcExample  -- \
       --testpath ~/tests --datadir /tests/config --filltests \
       --singletest 05_simpleTx


Test Source Code
----------------
This section explains **05_simpleTxFiller.yml**. I am only going to document
the things in which it is different from state transition tests.

State transition tests take their 
`genesis block <https://arvanaghi.com/blog/explaining-the-genesis-block-in-ethereum/>`_
from the client configuration (or, failing that, from the default client configuration)
in **retesteth**. In blockchain tests the values may be relevant to the test, so
you specify them directly.

::

  genesisBlockHeader:
    bloom: '0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000'
    coinbase: '0x8888f1f195afa192cfee860698584c030f4c9db1'
    difficulty: '131072'
    extraData: '0x42'
    gasLimit: '3141592'
    gasUsed: '0'
    mixHash: '0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421'
    nonce: '0x0102030405060708'
    number: '0'
    parentHash: '0x0000000000000000000000000000000000000000000000000000000000000000'
    receiptTrie: '0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421'
    stateRoot: '0xf99eb1626cfa6db435c0836235942d7ccaa935f1ae247d3f1c21e495685f903a'
    timestamp: '0x54c98c81'
    transactionsTrie: '0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421'
    uncleHash: '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347'

In a lot of existing tests you will see a definition for **sealEngine**. This is
related to getting a proof of work as part of the test. However, this is no longer
part of **retesteth**, so you can omit it or set it to **NoProof**.

::

  #  sealEngine: NoProof

Instead of a single transaction, we have a list of blocks. In a YAML list you 
tell different items apart by the dash character (**-**). The block list has two items in it.

::

  blocks:

The first block has one field, **transactions**, a list of transactions. 
Every individual transaction is specified with the same fields used in
state transition tests. This block only has one transaction, which transfers
10 Wei.

::

  - transactions:
    - data: ''
      gasLimit: '50000'
      gasPrice: 20


This is the **nonce** value for the transaction. The first value is the 
**nonce** associated with the address in the **pre:** section. 
Each subsequent transaction from the same address increments the **nonce**.

Alternatively, if you use **auto** for every transaction of an account, 
the retesteth tool will provide the nonce values automatically.

::

      nonce: '0'
      secretKey: 45a915e4d060149eb4365960e6a7a45f334393093061116b197e3240065ff2d8
      to: 0xde570000de570000de570000de570000de570000
      value: '10'

This is the second block. In contrast to the first block, in this one we specify
a **blockHeader** and override some of the default values.

::

  - blockHeader:
      gasLimit: '3141592'

A block can also contain references to `uncle blocks (blocks mined at the same
time) <https://www.investopedia.com/terms/u/uncle-block-cryptocurrency.asp>`_.
Note that writing tests with uncle headers is complicated, because you need
to run the test once to get the correct hash value. Only then can you put the
correct value in the test and run it again so it'll be successful.

::

    uncleHeaders: []


This block has two transactions.

::

    transactions:
    - data: ''
      gasLimit: '50000'
      gasPrice: '20'


This is another transaction from the same address, so the **nonce** is one more 
than it was in the previous one.

::

      nonce: '1'
      secretKey: 45a915e4d060149eb4365960e6a7a45f334393093061116b197e3240065ff2d8
      to: 0xde570000de570000de570000de570000de570000
      value: '20'
    - data: ''
      gasLimit: '50000'
      gasPrice: '30'


This transaction comes from a different address (addresses are uniquely derived
from the private key, and this one is different from the one in the previous
transaction). This transaction's **nonce** value is the initial value for 
that address, zero.

::

      nonce: '0'
      secretKey: 41f6e321b31e72173f8ff2e292359e1862f24fba42fe6f97efaf641980eff298
      to: 0xde570000de570000de570000de570000de570000
      value: '30'



.. _test-random-val:

Tests using the blockchain random value
---------------------------------------
Once Ethereum moves to proof of stake (PoS), there will no longer be any need for the block header 
fields **difficulty** and **mixHash**.
When the block header comes from a consensus client, the **mixHash** is a mostly random value that is 
produced by the beacon chain (the validators can each affect a bit on it, so it's not exactly random).
The **DIFFICULTY** opcode is no longer relevant either, so it is replaced by an opcode with the same 
value (0x44) called **PREVRANDAO**.
You can read more about this topic in `EIP-4399 <https://eips.ethereum.org/EIPS/eip-4399>`_.

In block tests we can simulate this value by specifying a **mixHash** as part of **blockHeader**.
However, the interaction of **mixHash** and **stateRoot** makes this process a bit complicated.

First, you write the test normally, using the block header field **mixHash** for the random value 
that in real execution would come from the consensus layer.
Note that **mixHash** has to be a 32 byte value.
Even if most of the bytes are zeros, you have to specify them.

When you run the test, it fails on the first block where the state is a function of the random value with an error that includes these lines:

::

   /retesteth/retesteth/TestOutputHelper.cpp(227): error: in "BlockchainTests/ValidBlocks/bcStateTests": 
   Error: Postmine block tweak expected no exception! Client errors with: 'Error importing raw rlp block: Block from pending block != t8ntool constructed block!
   Error in field: stateRoot
      .
      .
      .
   parentHash 0x76898c312aea29aa17df32e97399ccdf88e72c544305c9ddc3e76996e35ab951 vs 0x76898c312aea29aa17df32e97399ccdf88e72c544305c9ddc3e76996e35ab951
   receiptTrie 0x71043553dd2c4fbc22100a69d47ba3a790f7e428796792c552362b81e6cf5331 vs 0x71043553dd2c4fbc22100a69d47ba3a790f7e428796792c552362b81e6cf5331
   stateRoot 0x7a3760ed3aa3e40711b3ecd1cb898a9f37c14cbde7f95b7c5c7af05e6d794864 vs 0x1b5647d3ca49c4b0e9e57e113f85b1be28ac10f0577b6e70c76fb7d767949bf8

In the error there are two separate values of **stateRoot**.
The first, shown in red, is the expected value.
The second, shown in yellow, is the actual value.
You need to copy that second value into the block header.


::

    - blockHeader:
        mixHash: 0x0102030405060708091011121314151617181920212223242526272829303131
        stateRoot: 0x1b5647d3ca49c4b0e9e57e113f85b1be28ac10f0577b6e70c76fb7d767949bf8



If you use the random value also in another block, you repeat the process, once per block.

`You can see an example of this type of test here 
<https://github.com/ethereum/tests/blob/develop/src/BlockchainTestsFiller/ValidBlocks/bcStateTests/randomFiller.yml>`_.


Why is this procedure necessary?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
Retesteth was written back during the proof of work (PoW) days, when **mixHash** was a function of the 
**nonce**, which itself was produced from the completed block (including the post-block **stateRoot**).
The way that it fills the block header is to first get the block processed by the client, read the 
resulting **stateRoot** (as well as some other fields).
Then it reverts out of the block and sends it again, this time with the **blockHeader** fields and the 
calculated fields from the client.

This algorithm fails when the state, and therefore **stateRoot**, is affected by block header fields.

.. _invalid-block-tests:

Invalid Block Tests
===================

The invalid block test is in `tests/docs/tutorial_samples/06_invalidBlockFiller.yml 
<https://github.com/ethereum/tests/blob/develop/docs/tutorial_samples/06_invalidBlockFiller.yml>`_
We copy it to **bcExample**.


::
 
   mkdir ~/tests/src/BlockchainTestsFiller/InvalidBlocks/bcExample
   cp ~/tests/docs/tutorial_samples/06_* ~/tests/src/Bl*/In*/bcExample*
   cd ~
   ./dretesteth.sh -t BlockchainTests/InvalidBlocks/bcExample  -- \
       --testpath ~/tests --datadir /tests/config --filltests \
       --singletest 06_invalidBlock


Invalid block tests contain invalid blocks, blocks that
cause a client to raise an exception. To tell **retesteth** which exception 
should be raised by a block, we add an **expectException** field to the 
**blockHeader**. In that field we put the different forks the test 
supports, and the exception we expect to be raised in them. It is a good
idea to have a field that includes future forks.

::

  - blockHeader:
      gasLimit: '30'
    expectException:
      Berlin: TooMuchGasUsed
      '>=London': TooMuchGasUsed


.. warning::

   The **expectException** field is only used when **\\-\\-filltests** is specified.
   When it is not, **retesteth** just expects the processing of the block to fail,
   without ensuring the exception is the correct one. The reason for this feature 
   is that not all clients tell us the exact exception type when they reject a
   block as invalid.



Getting Exception Names
-----------------------
If you don't know what exception to expect, run the test without an **expectException**.
The output will include an error message similar to this one:

::

   Error: Postmine block tweak expected no exception! Client errors with: 
   'Error importing raw rlp block: Invalid gasUsed: header.gasUsed > header.gasLimit' 
   (bcBlockGasLimitTest/06_invalidBlock_Berlin, fork: Berlin, chain: default, block: 2)

Then took in **tests/conf/<name of client>/config** and look for the first few words
of the error message. For example, in **tests/conf/tn8tool/config** we find
this line:

::

      "TooMuchGasUsed" : "Invalid gasUsed:",

This tells us that the exception to expect is **TooMuchGasUsed**.


Transition Tests
================
Transition Tests start with one fork, which is used to execute blocks 1-4. 
Then, typically starting at block 5, it is the following fork.
You can see the list of `transition networks here <https://github.com/ethereum/retesteth/blob/develop/retesteth/configs/clientconfigs/t8ntool.cpp#L24-L33>`_ or in **.../tests/config/t8ntool/config**.

In the case of the ArrowGlacier to Merge transition it happens at a specific difficulty, 0x0C0000.
At the block difficulty most tests use, 0x020000, this happens on block 6.



Conclusion
==========
You should now be able to write most types of Ethereum tests. If you still have 
questions, you can look in the reference section or e-mail for help.
