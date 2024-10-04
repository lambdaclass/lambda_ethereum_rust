.. t8ntool-tutorial:

================================================================
Adding Transition Tool Support to your Execution Layer Client
================================================================

`Ori Pomerantz <mailto://qbzzt1@gmail.com>`_

.. note::
    This document is a tutorial. For reference on the **t8ntool** options
    `look here <https://ethereum-tests.readthedocs.io/en/latest/retesteth-ref.html>`__.


Why Do This?
=============
Typically you would add transition tools (t8ntool) support to a client to run the state 
transition and blockchain tests from `the standard repository <https://github.com/ethereum/tests>`_
using the `retesteth <./retesteth-tutorial.html>`_ tool.


Configuration
==============
The easiest way to configure a new **t8ntool** client is to start by duplicating the old one.

#. Run these commands (as root if necessary)

   ::

     cd tests/config
     cp -r t8ntool t8ntool2

#. Edit **tests/config/t8ntool2/start.sh**. You need two changes:

   - Change **/bin/evm t8n** and **/bin/evm t9n** to call your own executable.
   - If you need any outputs to help with debugging, add them to a log file

     ::
  
        echo t8ntool $* > /tests/t8ntool.log


#. Add your client to the docker image. To do that:

   #. `Recreate the docker container <https://ethereum-tests.readthedocs.io/en/latest/retesteth-tutorial.html#using-the-latest-version>`_.
   #. When you get to step 3, add the lines to download and compile your own client.


#. Run **retestheth** with **-\\-clients t8ntool2** to use your client.


Command line parameters
========================

If **t8ntool** is called with **-v**:

- This is the only parameter
- You are supposed to respond with a one line string that identifies the client
- This string goes into **<test>._info.filling-rpc-server** in the filled test files.


Otherwise, you are supposed to run an actal test (or part of one). In that case your parameters 
depends on the exact test type.


Transaction tests
-----------------
You can recognize these tests by the fact that there is no **-\\-input.alloc** field provided.

The command line looks like this:

::

    start.sh --input.txs /dev/shm/e79d0f66-4c95-4c23-bed3-3593126d4c3d/tx.rlp --state.fork Merged

Here is how you interpret the parameters:

================ ===================================== ==============
Parameter name   Value                                 More details
================ ===================================== ==============
-\\-input.txs    RLP encoded transactions file name    `see here <./t8ntool-ref.html#transaction-file>`_
-\\-state.fork   Fork to check against                 Istanbul, London, Merged, etc.
================ ===================================== ==============



State transition tests
---------------------------
The command line looks like this (but all in one line)

::

   start.sh --state.fork Merged 
        --input.alloc /dev/shm/a12b28a5-e0d2-48c5-9744-6428173af2a5/alloc.json 
        --input.txs /dev/shm/a12b28a5-e0d2-48c5-9744-6428173af2a5/txs.rlp 
        --input.env /dev/shm/a12b28a5-e0d2-48c5-9744-6428173af2a5/env.json 
        --output.basedir /dev/shm/a12b28a5-e0d2-48c5-9744-6428173af2a5 
        --output.result out.json 
        --output.alloc outAlloc.json


Optionally, there may be EVM tracing turned on, in which case you get the additional parameters:


::

        --trace 
        --trace.memory 
        --trace.returndata



The parameters are:

==================== ======================================= ==============
Parameter name       Value                                   More details
==================== ======================================= ==============
-\\-input.txs        RLP encoded transactions file name      `transaction files <./t8ntool-ref.html#transaction-file>`_
-\\-state.fork       Fork to check against                   Istanbul, London, Merged, etc.
-\\-input.alloc      Pretest allocation file with the state  `allocation files <./t8ntool-ref.html#allocation-files>`_
-\\-input.txs        Transactions file                       `same as for -\\-input.txs <./t8ntool-ref.html#transaction-file>`_
-\\-input.env        Environment file
-\\-output.basedir   Directory to write output files
-\\-output.result    Test output                             `results file <./t8ntool-ref.html#result-file>`_
-\\-output.alloc     Posttest allocation file with the state `same as for -\\-input.alloc <./t8ntool-ref.html#allocation-files>`_
-\\-trace            None (a flag)
-\\-trace.memory     None (a flag)
-\\-trace.returndata None (a flag)
==================== ======================================= ==============



Traces
^^^^^^
If **-\\-trace** is specified, you need to reate a file (or files) called 
trace-<transaction number>-<transaction hash>.jsonl. The format of this file is specified in 
`EIP 3155 <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-3155.md>`_.

This file should include the content of the memory only if **-\\-trace.memory** is specified.
It should include the content of the return buffer only if **-\\-trace.returndata** is specified.



Blockchain tests
---------------------------------------
Blockchain tests are very similar to state transition tests, with these differences:

- There is an additional parameter, **-\\-state.reward**, which specifies the block reward.
  Post merge this value is still specified, but it is zero.

- The same test could run **t8ntool** multiple times, once per block.



Minimal t8ntool client
======================
This is a minimal **t8ntool** client, written in Python (the retesteth docker image already has Python, 
and we'd need to change it to get Node.js). It writes the two files that **t8ntool** requires, the
output file and the output allocations file. The values it writes are nonsensical, but they pass the 
minimal requirements.



::

   #! /usr/bin/python

   import sys
   import shutil

   conf = {}

   def parseParams(argv):
      """parse the parameters to the script"""
      for i in range(len(argv)):
         if argv[i][:2] == "--":
            conf[argv[i]] = argv[i+1]


   parseParams(sys.argv)

   # No change in the blockchain state
   shutil.copyfile(conf["--input.alloc"], conf["--output.basedir"] + "/" + conf["--output.alloc"])


   # Read the environment
   envFile = open(conf["--input.env"])
   envJSON = envFile.read()
   envFile.close()

   # Write the output
   resFile = open(conf["--output.basedir"] + "/" + conf["--output.result"], "w")
   resFile.write("""
     {
       "currentDifficulty" : "0x020000",
       "logsBloom": "0x""" + '01'*256 + """",
       "logsHash": "0x0102030405060708091011121314151617181920212223242526272829303132",
       "receipts": [],
       "receiptsRoot": "0x0102030405060708091011121314151617181920212223242526272829303132",
       "stateRoot": "0x0102030405060708091011121314151617181920212223242526272829303132",
       "txRoot": "0x0102030405060708091011121314151617181920212223242526272829303132"
     }
   """)
   resFile.close()
