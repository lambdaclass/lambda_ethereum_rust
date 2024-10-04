
Post Section
============

::

        "post" : {
            "London" : [
                {
                    "indexes" : {
                        "data" : 0,
                        "gas" : 0,
                        "value" : 0
                    },
                    "hash" : "0xe4c855f0d0e96d48d73778772ee570c45acb7c57f87092e08fed6b2205d390f4",
                    "logs" : "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
                    "txbytes" : "0x02f88d0101808207d0871000000000000094cccccccccccccccccccccccccccccccccccccccc80a4693c61390000000000000000000000000000000000000000000000000000000000000000c001a05fecc3972a35c9e341b41b0c269d9a7325e13269fb01c2f64cbce1046b3441c8a07d4d0eda0e4ebd53c5d0b6fc35c600b317f8fa873b3963ab623ec9cec7d969bd"
                    "expectException" : "TR_IntrinsicGas"
                }
            ]
        },

Post section is a map `<FORK> => [TransactionResults]`

The test can have many fork results and each fork result can have many transaction results.

In generated test indexes are a single digit and could not be array. Thus define a single transaction from the test.
See transaction section which define transactions by `data`, `gasLimit`, `value` arrays.


**Fields**

======================= ===============================================================================
``London``               fork name as defined by client config (test standard names)
``indexes``              define an index of the transaction in txs vector that has been used for this result
``data``                 index in transaction data vector
``gas``                  index in transaction gas vector
``value``                index in transaction value vector
``hash``                 hash of the post state after transaction execution
``logs``                 log hash of the transaction logs
``txbytes``              the transaction bytes of the generated transaction
``expectException``      for a transaction that is supposed to fail, the exception
======================= ===============================================================================
