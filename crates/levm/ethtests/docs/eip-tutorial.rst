.. eip-tests-tutorial:

###########################################
Testing EIPs
###########################################
`Ori Pomerantz <mailto://qbzzt1@gmail.com>`_

In this tutorial you learn how to write tests for a new EIP 
(after the EIP itself has been implemented on a branch of geth).


Environment
===========
The easiest way to do this is to run **restetheth** `in a docker container you build <retesteth-tutorial.html#using-the-latest-version>`_.
To be able to isolate problems, it is best if the docker container includes both the branch geth and the standard one.

#. Get the **Dockerfile** and the script:

   ::

      mkdir ~/retestethBuild
      cd ~/retestethBuild
      wget https://raw.githubusercontent.com/ethereum/retesteth/develop/dretesteth.sh
      chmod +x dretesteth.sh
      wget https://raw.githubusercontent.com/ethereum/retesteth/develop/Dockerfile

#. Edit **Dockerfile**:

   * In the last line of the string of commands that builds **geth**, remove the **&& rm -rf /usr/local/go**.
     We are going to need to compile **geth** again in a moment.

      ::

         RUN cd /geth && apt-get install wget \
           && wget https://dl.google.com/go/go1.18.linux-amd64.tar.gz \
           && tar -xvf go1.18.linux-amd64.tar.gz \
           && mv go /usr/local && ln -s /usr/local/go/bin/go /bin/go \
           && make all && cp /geth/build/bin/evm /bin/evm \
           && cp /geth/build/bin/geth /bin/geth \
           && rm -rf /geth  

   * Duplicate the **geth** commands, except for these changes:
    
      * Clone a repository that includes the modified geth (it may be a branch of the main geth repository, or a different repository altogether).
      * Remove the code that installs the Go programming language.
      * Change the binaries to **evm-eip** and **geth-eip**.

      |

      For example, there is a version of **geth** `here <https://github.com/snreynolds/go-ethereum>`_ with EIP-1153 support.
      These are the commands to install and compile it:

      ::

          RUN git clone --depth 1 https://github.com/snreynolds/go-ethereum /geth
          RUN cd /geth && apt-get install wget \
              && make all && cp /geth/build/bin/evm /bin/evm-eip \
              && cp /geth/build/bin/geth /bin/geth-eip \
              && rm -rf /geth && rm -rf /usr/local/go

#. Issue **./dretesteth.sh build**.

#. Run at least one test to initialize the **tests/config** directory.

#. To make life easier, change the ownership of those files:

   ::

      sudo find tests/config -exec chown `whoami` {} \;


#. Copy the **t8ntool** configuration to **t8ntool-eip**:

   ::
   
      cd tests/config
      cp -R t8ntool/ t8ntool-eip

#. Edit **t8ntool-eip/start.sh** to use the **evm-eip** binary:

   ::

      #!/bin/sh

      if [ $1 = "-v" ]; then
          /bin/evm-eip -v
      else
          stateProvided=0
          for index in ${1} ${2} ${3} ${4} ${5} ${6} ${7} ${8} ${9} ${10} ${11} ${12} ${13} ${14} ${15} ${16} ${17} $
              if [ $index = "--input.alloc" ]; then
                  stateProvided=1
                  break
              fi
          done
          if [ $stateProvided -eq 1 ]; then
              /bin/evm-eip t8n ${1} ${2} ${3} ${4} ${5} ${6} ${7} ${8} ${9} ${10} ${11} ${12} ${13} ${14} ${15} ${16$
          else
              /bin/evm-eip t9n ${1} ${2} ${3} ${4} ${5} ${6} ${7} ${8} ${9} ${10} ${11} ${12} ${13} ${14} ${15} ${16$
          fi
      fi


#. Use **-\\-clients t8ntool-eip** to run tests with the modified geth. 


Test Cases
================
Most EIPs include multiple test cases, some valid, some not.
In most cases you'll be able to write either state tests or block tests to verify the functionality.


Testing new Opcodes
====================
The `Yul <https://docs.soliditylang.org/en/v0.8.15/yul.html>`_ programming language supports 
`verbatim opcodes <https://docs.soliditylang.org/en/v0.8.15/yul.html#verbatim>`_. 
See here for `an example of using verbatim <https://github.com/ethereum/tests/blob/develop/src/GeneralStateTestsFiller/stEIP1559/baseFeeDiffPlacesFiller.yml#L39>`_
(written before the `BASEFEE <https://www.evm.codes/#48>`_ opcode was supported by Yul).



Conclusion
==========
At this point you should know enough to test whether geth implements an EIP
correctly or not.

