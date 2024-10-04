.. _t8ntool_ref:

###########################################
Transition Tool
###########################################



Command Line Parameters
=======================
The command line parameters are used to specify the parameters, input files, and
output files.

In the **t8ntool** client provided with the system, which uses **geth**, the commands being called are:

- **evm t8n** For state transition and blockchain tests.
- **evm t9n** For transaction tests.

However, you can change that by editing the **tests/config/t8ntool/start.sh** file.


Test Parameters
---------------
- **-\\-state.fork** *fork name*
- **-\\-state.reward** *block mining reward* (appears only in Block tests)
- **-\\-trace** produce a trace

Input Files
-----------
- **-\\-input.env** *full path to environment file*
- **-\\-input.alloc** *full path to pretest allocation file with the state*
- **-\\-input.txs** *full path to transaction file*

State transition and blockchain tests have all three input file parameters.
Transaction tests, which only test transaction parsing, only have the **-\\-input.txs** parameter.

.. note::

   If you want to specify any of this information in `stdin`, either 
   omit the parameter or use the value **stdin**.

Output Files
------------
- **-\\-output.basedir** *directory to write the output files*
- **-\\-output.result** *file name for test output*
- **-\\-output.alloc**  *file name for post test allocation file*



.. note::

   If you want to receive this information into `stdout`, either omit
   the parameter or use the value **stdout**.


File Structures
===============
Most of the t8ntool files are in JSON format. Any values that are not
provided are assumed to be zero or empty, as applicable.


Transaction File
----------------
This file is a single line `"0x<rlp encoded transaction><rlp encoded transaction>..."`.
If there are no transactions, the line is `"0xc0"`.
This is an input to the tool, which `retesteth` calls `txs.rlp` for state transition and blockchain tests and `tx.rlp` for transaction tests.


Environment File
----------------
This file is a map with the execution environment. 
This is an input to the tool, which `retesteth` calls `env.json`.
It has these fields:


* `currentCoinbase`
* `currentDifficulty`
* `currentGasLimit`
* `currentNumber`
* `currentTimestamp`
* `previousHash`, the hash of the previous (`currentNumber`-1) block
* `blockHashes`, a map of historical block numbers and their hashes

.. note::

   Some tests include multiple blocks. In that case, the test software runs 
   `t8ntool` multiple times, one per block. 


Example
^^^^^^^

::

  {
     "currentCoinbase" : "0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba",
     "currentDifficulty" : "0x020000",
     "currentGasLimit" : "0x05f5e100",
     "currentNumber" : "0x01",
     "currentTimestamp" : "0x03e8",
     "previousHash" : "0xe729de3fec21e30bea3d56adb01ed14bc107273c2775f9355afb10f594a10d9e",
     "blockHashes" : {
         "0" : "0xe729de3fec21e30bea3d56adb01ed14bc107273c2775f9355afb10f594a10d9e"
     }
  }


Allocation Files
----------------
These files show the state of various accounts and contracts on the blockchain.
In `retesteth` there are two of these files:
`alloc.json` which is the input state and `outAlloc.json`
which is the output state.

The file is a map of `address` values to account information. The account 
information that can be provided is:

* `balance` 
* `code` (in machine language format)
* `nonce`
* `storage`

Example
^^^^^^^

::

   {
       "a94f5374fce5edbc8e2a8697c15331677e6ebf0b": {
           "balance": "0x5ffd4878be161d74",
           "code": "0x5854505854",
           "nonce": "0xac",
           "storage": {
              "0x0000000000000000000000000000000000000000000000000000000000000000": 
              "0x0000000000000000000000000000000000000000000000000000000000000004"
           }
        },
        "0x8a8eafb1cf62bfbeb1741769dae1a9dd47996192":{
           "balance": "0xfeedbead",
           "nonce" : "0x00"
        }
   }






Result File
-----------
In `retesteth` this file is called `out.json`. It is the post state after 
processing the block. It should include the following fields:

* `stateRoot`
* `txRoot`
* `receiptRoot`
* `logsHash`
* `logsBloom`, the `bloom filter <https://en.wikipedia.org/wiki/Bloom_filter>`_ for
  the logs.
* `receipts`, a list of maps, one for each transaction, with the transaction receipt.
  Each of those receipts includes these fields:

  * `root`
  * `status`
  * `cumulativeGasUsed`
  * `logsBloom`
  * `logs`
  * `transactionHash`
  * `contractAddress`, the address of the created contract, if any
  * `gasUsed`
  * `blockHash`, all zeros because this is created before the block is finished
  * `transactionIndex`


Example
^^^^^^^

::

   {
     "stateRoot": "0x1c99b01120e7a2fa1301b3505f20100e72362e5ac3f96854420e56ba8984d716",
     "txRoot": "0xb5eee60b45801179cbde3781b9a5dee9b3111554618c9cda3d6f7e351fd41e0b",
     "receiptRoot": "0x86ceb80cb6bef8fe4ac0f1c99409f67cb2554c4432f374e399b94884eb3e6562",
     "logsHash": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
     "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
     "receipts": [
        {
            "root": "0x",
            "status": "0x1",
            "cumulativeGasUsed": "0xa878",
            "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "logs": null,
            "transactionHash": "0x4e6549e2276d1bc256b2a56ead2d9705a51a8bf54e3775fbd2e98c91fb0e4494",
            "contractAddress": "0x0000000000000000000000000000000000000000",
            "gasUsed": "0xa878",
            "blockHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "transactionIndex": "0x0"
        }
     ]
   }



Trace Files
-----------
If **-\\-trace** is specified, the t8ntool creates a file (or files) called 
`trace-<transaction number>-<transaction hash>.jsonl`. The format of this file
is specified in 
`EIP 3155 <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-3155.md>`_.

If the transaction fails and does not produce a hash, the name of the file is
still `trace-<transaction number>-<value that is a legitimate hash>.jsonl`.



Using Standard Input and Output
===============================
It should also be possible to run a `t8ntool` with input coming from `stdin`
and output going to `stdout`. In this case, the input is all one object and
the output is all one object.


Input
-----
When the input is provided using `stdin`, it can have any combination 
of these three fields (whichever ones aren't provided in file form)

* `txs`, a list of transactions
* `alloc`, a map of the pretest accounts
* `env`, a map of the execution environment


Output
------
When the output goes to `stdout`, it can have any combination of these fields
(whichever ones don't have a specified output file):

* `result`, the post state (the blockchain state after processing)
* `body`, the transactions and their results 

