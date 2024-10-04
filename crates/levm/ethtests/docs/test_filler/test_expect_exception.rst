This field specifies the exception we expect to see raised by for this 
transaction. It is optional - you only add it if an exception is expected.

  .. list-table::
     :header-rows: 1

     * - JSON

       - YAML

     * -

         ::

            "expectException": {
               ">=London": TR_TipGtFeeCap
	    }

       -

         ::

            expectException:
               ">=London": TR_TipGtFeeCap


The fields are fork specifications:



  .. list-table::
     :header-rows: 1


     * -

         Type of specification

       -

         Example


     * -

         Specific fork

       -

         Berlin


     * -

         A specific fork and all forks after it

       -

         >=London


     * -

         Anything prior to a specific fork (not including that fork)

       -

         <Berlin


The value is an exception name. You can see the list 
`in the retesteth code 
<https://github.com/ethereum/retesteth/blob/develop/retesteth/configs/clientconfigs/t8ntool.cpp#L158-L166>`_.
