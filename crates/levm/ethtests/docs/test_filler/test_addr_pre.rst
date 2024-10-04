
.. include:: ../test_filler/test_addr.rst

- **code**:

  The code of the contract. There are several possibilities:

  - If the account is not a contract, this value is **0x**

  - Raw virtual machine code. This is for cases where it is impossible to
    provide source code, or the source code is in a language retesteth
    does not recognize, such as `Vyper <https://vyper.readthedocs.io/en/stable/>`_.

    ::
 
      :raw 0x600160010160005500

  - `Lisp Like Language (lll) <http://blog.syrinx.net/the-resurrection-of-lll-part-1/>`_, 
    for example:
   
    ::

       {
          ; Add 2+2 and store the value in storage location 0
          [[0]] (ADD 2 2)
       }

  - `Yul, which is documented here <https://docs.soliditylang.org/en/v0.8.3/yul.html>`_, 
    for example:

    ::

       :yul {
          // Add 2+2 and store the value in storage location 0
          sstore(0, add(2,2))
       }

    Optionally, you can specify the hard fork for which to compile the code

    ::

       :yul berlin {
          // Add 2+2 and store the value in storage location 0.
          // Because we compile using the Berlin hardfork, 
          // there is no PUSH0, and we can use the code to check
          // forks prior to Shanghai.
          sstore(0, add(2,2))
       }
 


  - `Solidity, which you can learn here <https://cryptozombies.io/>`_. Solidity
    code can be provided to a test in two ways:
  
    - Put the solidity code itself in the contract definition (same place as 
      the LLL or Yul code).

    - Put a **:solidity** section with the contract source code. In 
      that case, the value in **code:** is **:solidity <name of contract>**.

    In either case, you can specify the hardfork to use using this syntax:

    ::

        // RETESTETH_SOLC_EVM_VERSION=berlin
