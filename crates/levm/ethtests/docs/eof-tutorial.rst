.. eof_tutorial:

###########################################
Ethereum Object Format Tests
###########################################

`Ori Pomerantz <mailto://qbzzt1@gmail.com>`_

In this tutorial you learn how to write and execute EOF tests.
These tests let you check various combinations to see what is accepted as valid EOF
and what is rejected.

Make sure you understand `State Transition Tests <state-transition-tutorial.html>`_ 
before you start here.


Fillers
=======
The fillers for EOF tests are in **.../src/EOFFiller**. 
This tutorial explains the YML filler, 
**.../src/EOFFiller/efExample/ymlExampleFiller.yml**.


Overall Structure
-----------------
The file includes these sections:

- **_info**: Human readable comments.
- **data**:  A list of entries.
  Each entry is init code that can create a contract
- **expect**: The expected results.


The Data Section
----------------
Each entry in the data is typically **:raw** bytes, because we are checking a data 
format. However, because EOF is a lot more complicated than most raw data 
provided in tests, it is a good idea to use multi-line fields with comments.
For example, this code

::

  - | 
      :raw 
      0xEF0001  # Magic and version
        010004  # One code segment
        020001  # One code segment
          000a  # Code segment zero length: 10 bytes
        030016  # Data segment length (the code being deployed): 0x16=22 bytes
        00      # End of header 

This is functionally equivalent to

::

  - :raw 0xEF0001010004020001000a03001600

But a lot more readable.



The Expect Section
------------------
Here is a sample expect section entry.

::

  - indexes:
      data:
      - 0-1
    network: 
    - '>=Shanghai'
    result: !!bool true

It is very similar to the expect section of a state transition test, except for
these differences:

- In the **indexes** subsection there is only **data**.
  These tests don't have gas or value fields to match.

- The **result** can only be one of two values:

  - **!!bool true** if the contract is supposed to get created

  - **!!bool false** if contract creation is supposed to fail


