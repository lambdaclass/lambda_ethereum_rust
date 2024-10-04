
.. note::
   Except for the values in the **indexes** section, all the field values in the 
   tests' source code are strings.
   In YAML string is the default field type. In JSON string values are enclosed by
   quotes.

   When a value is numeric, such as a value 
   in storage or an address's balance, it can be specified either in 
   hexademical (starting with **0x**), or in decimal (starting with a digit).
   For the sake of legibility, numeric values can also have underscores. For 
   example, you can use **1_000_000_000** for 10^9, which is a lot more readable
   than **1000000000**.

   When a numeric field exceeds 256 bits, you can specify it using the syntax
   **0x:bigint 0x1000....00001**. 
   
