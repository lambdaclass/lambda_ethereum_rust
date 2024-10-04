.. custom-compiler-tutorial:

================================================================
Custom compiler support
================================================================

`Ori Pomerantz <mailto://qbzzt1@gmail.com>`_

In this tutorial you learn how to use a custom compiler with retesteth tests.
We do this by following the steps to write and execute a test written in
`the Huff programming language <https://github.com/huff-language/huff-rs>`_.


Why Do This?
=============
Sometimes it is convenient to write a test using a different language than the three supported ones
(`LLL <https://lll-docs.readthedocs.io/en/latest/lll_introduction.html>`_, 
`Solidity <https://docs.soliditylang.org/en/v0.8.15/solidity-by-example.html>`_, 
and `Yul <https://docs.soliditylang.org/en/v0.8.15/yul.html>`_). 
While such tests are unlikely to be accepted as standard tests, they can help debug client changes.



Install Huff as part of the Docker
=======================================
One way to do this is to run **restetheth** 
`in a docker container you build <retesteth-tutorial.html#using-the-latest-version>`_.

#. Get the **Dockerfile** and the script:

   ::

      mkdir ~/retestethBuild
      cd ~/retestethBuild
      wget https://raw.githubusercontent.com/ethereum/retesteth/develop/dretesteth.sh
      chmod +x dretesteth.sh
      wget https://raw.githubusercontent.com/ethereum/retesteth/develop/Dockerfile

#. Edit **Dockerfile**:

   * On line 1 change the original image from Ubuntu 18.04 to Ubuntu 20.04.
     This step is necessary because the C libraries on Ubuntu 18.04 are too old for the Huff compiler.

      ::
     
         FROM ubuntu:20.04 as retesteth

   * On the line that downloads the **retesteth** source from github change the branch from master 
     to **develop**.

      ::

          RUN git clone --depth 1 -b develop https://github.com/ethereum/retesteth.git /retesteth

   * Before the entry point definition add a command to download and configure the Huff compiler.

      ::

          # Huff compiler
          RUN curl -L get.huff.sh | bash \
              && ~/.huff/bin/huffup

#. Issue **./dretesteth.sh build**.

   You will receive these errors. 
   Ignore them, they are merely an artifact of **npm** and **yarn** not being installed on the Docker image.

    ::

        /root/.huff/bin/huffup: line 18: npm: command not found
        huffup: warning: It appears your system has an outdated installation of huffc via npm.
        huffup: warning: Uninstalling huffc with npm to allow huffup to take precedence...
        /root/.huff/bin/huffup: line 21: npm: command not found
        /root/.huff/bin/huffup: line 25: yarn: command not found



Update the client configuration
====================================
Custom compiler information is provided as part of the client configuration.


#. Run at least one test to initialize the **tests/config** directory.

#. To make life easier, change the ownership of those files:

   ::

      sudo find tests/config -exec chown `whoami` {} \;

#. Copy the **t8ntool** configuration to **t8ntool-huff**:

   ::
   
      cd tests/config
      cp -R t8ntool/ t8ntool-huff


#. Edit **tests/config/t8ntool-huff/config** to change the **customCompilers** definition:

   ::

        "customCompilers" : {
            ":huff" : "huff.sh"
        },

#. Create a file, **tests/config/t8ntool-huff/huff.sh**, to call the Huff compiler

   ::

      #!/bin/sh
      # You can call a custom executable here
      # The code src comes in argument $1 as a path to a file containing the code
      # So if you have custom compiler installed in the system the command would look like:
      # mycompiler $1

      mv $1 $1.huff
      code=`~/.huff/bin/huffc $1.huff -r | grep -v Compiling`
      echo 0x$code
      rm $1.huff

      # Make sure your tool output clean bytecode only with no log or debug messages
      # echo "0x600360005500"



Create a test file that uses Huff
====================================
Use this syntax for the **code:** definition of a contract, such as:

::

      code: |
        :huff
          #define macro MAIN() = takes(0) returns(0) {
             0x01 0x01 add
             0x00 sstore
             stop
          }


The **:huff** keyword matches the one in **tests/config/t8ntool-huff/config**, 
so the **retesteth** tool knows to call **huff.sh**. 
It is followed by the Huff code.

You can see a sample test `here 
<https://github.com/ethereum/tests/blob/develop/docs/tutorial_samples/13_huffFiller.yml>`_.
