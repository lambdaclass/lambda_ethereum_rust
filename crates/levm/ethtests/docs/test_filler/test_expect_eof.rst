Expect
======
This section contains the information we expect to see after the test is 
concluded.


Format
------

.. list-table::
   :header-rows: 1

   * - JSON

     - YAML

   * -

       ::

           {
              "name-of-test": {
                 <other sections>,
                 "expect": [
                   {
                      "indexes": {
                        "data": [0, "2-3", ":label foo"],
                      },
                      "network": ["Istanbul", <other forks, see below>],
                      "result": true
                   { <forks & results> }
                 ]
              }
           }          


     -

       ::

           name-of-test:
              <other sections>
              expect:
              - indexes:
                  data:
                  - !!int 0
                  - 2-3
                  - :label foo
                network:
                - Istanbul
                - <another fork>
                result: !!bool true
              - <forks & results>


The Network Specification
-------------------------
The string that identifies a fork (version) within a **network:** 
list is one of three option:

- The specific version: **Istanbul**
- The version or anything later: **>=Frontier**
- Anything up to (but not including) the version **<Constantinople**



The Indexes
-----------
The **data** values which are covered by this **expect** item.
Each **data** value uses one of these formats:

.. list-table::
   :header-rows: 1

   * - JSON

     - YAML

     - Meaning

   * - -1
 
     - !!int -1
  
     - All the (**data**, **gas**, or **value**) values in the transaction

   * - <n>

     - !!int <n>

     - The n'th value in the list (counting from zero)

   * - "<a>-<b>"

     - a-b

     - Everthing from the a'th value to the b'th value (counting from zero)

   * - ":label foo"

     - :label foo

     - Any value in the list that is specified as **:label foo <value>**


The Result
-----------
Whether the data should result in a successful contract deployment or not.

.. list-table::
   :header-rows: 1

   * - JSON

     - YAML

     - Meaning

   * - true
 
     - !!bool true
  
     - Successful deployment

   * - false

     - !!bool false

     - Failed deployment

