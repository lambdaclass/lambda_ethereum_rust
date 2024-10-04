Transaction
=============

This is the data to be deployed.


Format
------------


.. list-table::
   :header-rows: 1

   * - JSON

     - YAML

   * -

       ::

           {
               "name-of-test": {
                  <other sections>,
                  "data": [
                     ":raw 0x112200",
                     ":raw 0x223344"
                  ]
           }

     - ::

           name-of-test:
             <other sections>
             data:
             - :raw 0xBAD060A7
             - |
               :raw # or cooked
               0xBAD0 # or good
                 60A7 


