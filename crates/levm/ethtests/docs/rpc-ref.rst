.. rpc_ref:

#######################
The RPC Interface
#######################

Some clients, such as `besu <https://www.hyperledger.org/use/besu>`_, run tests 
using this interface. This allows the client to run anywhere there is connectivity
to the system running **retesteth**.

In addition to requiring some of the `standard Ethereum RPC function
<https://ethereum.org/en/developers/docs/apis/json-rpc/#top>`_, **retesteth**
requires some specific functions to setup and execute tests.


Retesteth-Specific RPCs
=======================

debug_accountRange
------------------
Get a list of accounts at a certain point in time.

Parameters
^^^^^^^^^^
#. **string _blockHashOrNumber**: The hash or number of the block
#. **int _txIndex**: Transaction index for the point in which we want the list of accounts
#. **string _addressHash**: The hash at which to start the list
   If **_maxResults** is equal to the number of accounts or more than that then 
   we receive all the addresses
   and there is no problem. But if there are too many accounts to report them all, we 
   receive the next hash at which we can find an address. We then call this method again,
   with that value in **_addressHash**, to get the next batch of addresses.
#. **int _maxResults**: Maximum number of results

Result
^^^^^^^^^^^^
- **addressMap**: An object with hash values and the addresses they represent. We use
  the hashes (both here and in the **_addressHash** parameter) because that is the
  order in which addresses are stored in the client, so the easiest order to for
  paged retrieval.
- **nextKey**: The next hash (in case there are more addresses to 
  return than **_maxResults**.

Sample Request
^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "method": "debug_accountRange",
      "params": [
        "1",
        1,
        "0x0000000000000000000000000000000000000000000000000000000000000001",
        10
      ],
      "id": 9
    }

This request came from a state transition test, which means that there is only
one block and within it only one transaction.

#. **_blockHashOrNumber**: The block number, which is one (the only block there is)
#. **_txIndex**: We want the state after one transaction (the only transaction
   in the block)
#. **_addressHash**: This is the first request, so we want to start at the
   beginning.
#. **_maxResults**: We want up to ten results.


Sample Result
^^^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "id": 9,
      "result": {
        "addressMap": {
          "0x03601462093b5945d1676df093446790fd31b20e7b12a2e8e5e09d068109616b": "0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b",
          "0x0fbc62ba90dec43ec1d6016f9dd39dc324e967f2a3459a78281d1f4b2ba962a6": "0x095e7baea6a6c7c4c2dfeb977efac326af552d87",
          "0x9d860e7bb7e6b09b87ab7406933ef2980c19d7d0192d8939cf6dc6908a03305f": "0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba"
        },
        "nextKey": "0x0000000000000000000000000000000000000000000000000000000000000000"
      }
    }


- **addressMap**: Three entries, there are three addresses with meaningful information.
- **nextKey**: Zero, because there are no more results to return.


debug_storageRangeAt
--------------------
Get a list of storage values.

Parameters
^^^^^^^^^^
#. **string _blockHashOrNumber**: The hash or number of the block
#. **int _txIndex**: Transaction index for the point in which we want the list of accounts
#. **string _address**: Read storage values for this address.
#. **string _begin**: Start from this hash
#. **int _maxResults**: Maximum number of results

Result
^^^^^^^^^^^^
- **storage**: An object with hash values, and for each of them the key and value it
  represents.
- **complete**: Boolean value, true if this completes the storage entries.

Sample Request
^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "method": "debug_storageRangeAt",
      "params": [
        "1",
        1,
        "0x095e7baea6a6c7c4c2dfeb977efac326af552d87",
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        20
      ],
      "id": 17
    }


#. **string _blockHashOrNumber**: One, the only valid value for a state test
#. **int _txIndex**: One, the only valid value for a state test
#. **string _address**: An address
#. **string _begin**: Start from the beginning, zero
#. **int _maxResults**: Read up to twenty results


Sample Result
^^^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "id": 17,
      "result": {
        "storage": {
          "0x290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563": {
            "key": "0x00",
            "value": "0x02"
          },
          "0x8a8c65155279fdd366bbe4502fff15c2162ef3f469afd7533efe047403a26923" : {
            "key" : "0x60a7",
            "value" : "0x60a7"
          }
        },
        "complete": true
      }
    }

- **storage**: An object with two hash values, each of which has the key and value that
  it represents.
- **complete**: True, this is the entire storage.


debug_traceTransaction
----------------------
Get the virtual machine trace of a transaction. Not currently implemented.


test_mineBlocks
---------------
Put the existing valid transactions into the current block and finish it, and create
a number of blocks after it.

Parameters
^^^^^^^^^^
#. **int _number**: The number of blocks to create after the current block.

 
Result
^^^^^^^^^^^^
Boolean value, **true** if successful 

Sample Request
^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "method": "test_mineBlocks",
      "params": [
        1
      ],
      "id": 28
    }

Create one additional block

Sample Result
^^^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "id": 28,
      "result": true
    }

Success


test_modifyTimestamp
--------------------

Parameters
^^^^^^^^^^
#. **int _timestamp**: The new timestamp

Result
^^^^^^^^^^^^
Boolean value, **true** if successful 

Sample Request
^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "method": "test_modifyTimestamp",
      "params": [
        1000
      ],
      "id": 2
    }

Change the timestamp to 1000. This value is a `Unix timetamp
<https://www.unixtimestamp.com/>`_, 1000 second after midnight
on January 1st, 1970, GMT.

Sample Result
^^^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "id": 2,
      "result": true
    }

Success


test_rewindToBlock
------------------
Revert the state of the blockchain to a specific block number.
Cancel the blocks after it, which lets us run multiple tests without having to 
setup a new genesis block for each one.


Parameters
^^^^^^^^^^
#. **int _block**: The number of the last block that is not cancelled. If it is
   the genesis block, this value is zero.


Result
^^^^^^^^^^^^
Boolean value, **true** if successful 

Sample Request
^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "method": "test_rewindToBlock",
      "params": [
        0
      ],
      "id": 22
    }

Rewind all the way to the genesis block.


Sample Result
^^^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "id": 22,
      "result": true
    }

Success


test_setChainParams
-------------------
This method tells a client to initialize a test chain to a given state.

Parameters
^^^^^^^^^^
An object that contains the chain parameters for the test:

- **params**: Chain parameters:
  - **chainID**: The chain identifier.
  - **<fork>ForkBlock**: The block in which that fork starts on this chain.
- **accounts**: The accounts at the test's start. This is an object whose
  keys are the addresses of the accounts. For each account there are these
  parameters (all the scalar values are strings with a hexadecimal number in them):
  - **balance**: Balance in wei
  - **code**: The EVM code (**0x** if there is none).
  - **nonce**: The nonce for the next transaction from this address.
  - **storage**: An object with keys and their values.
- **sealEngine**: Currently always **NoReward**.
- **genesis**: The parameters of the genesis block.


Result
^^^^^^^^^^^^
Boolean value, **true** if successful 

Sample Request
^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "method": "test_setChainParams",
      "params": [
        {
          "params": {
            "homesteadForkBlock": "0x00",
            "EIP150ForkBlock": "0x00",
            "EIP158ForkBlock": "0x00",
            "byzantiumForkBlock": "0x00",
            "constantinopleForkBlock": "0x00",
            "constantinopleFixForkBlock": "0x00",
            "istanbulForkBlock": "0x00",
            "berlinForkBlock": "0x00",
            "chainID": "0x01"
          },
          "accounts": {
            "0x095e7baea6a6c7c4c2dfeb977efac326af552d87": {
              "balance": "0x0de0b6b3a7640000",
              "code": "0x600160010160005500",
              "nonce": "0x00",
              "storage": {}
            },
            "0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba": {
              "balance": "0x00",
              "code": "0x",
              "nonce": "0x01",
              "storage": {}
            },
            "0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b": {
              "balance": "0x0de0b6b3a7640000",
              "code": "0x",
              "nonce": "0x00",
              "storage": {}
            }
          },
          "sealEngine": "NoReward",
          "genesis": {
            "author": "0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba",
            "difficulty": "0x020000",
            "gasLimit": "0xff112233445566",
            "extraData": "0x00",
            "timestamp": "0x00",
            "nonce": "0x0000000000000000",
            "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
          }
        }
      ],
      "id": 1
    }

Sample Result
^^^^^^^^^^^^^^^^

::

    {
      "jsonrpc": "2.0",
      "id": 1,
      "result": true
    }

Success

Standard RPCs Retesteth Uses
============================
- `eth_blockNumber <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_blocknumber>`_
- `eth_getBalance <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_getbalance>`_
- `eth_getBlockByNumber <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_getblockbynumber>`_
- `eth_getCode <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_getcode>`_
- `eth_getTransactionCount <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_gettransactioncount>`_
- `eth_sendRawTransaction <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_sendrawtransaction>`_
- `web3_clientVersion <https://ethereum.org/en/developers/docs/apis/json-rpc/#web3_clientversion>`_



