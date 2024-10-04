.. _retesteth_install:

These directions are written using Debian Linux 11 on Google Cloud
Platform (using a 20 GB disk - the default 10 GB is not enough), 
but should work with minor changes on any other version of
Linux running anywhere else with an Internet connection.

#. Install docker. You may need to reboot afterwards to get the latest
   kernel version.

   ::

      sudo apt install -y wget docker docker.io

#. Download the latest **retesteth** `docker image <https://retesteth.ethdevops.io/dretesteth.tar>`_

   ::

      wget http://retesteth.ethdevops.io/dretesteth.tar

#. Load the docker image: 

   ::

      sudo docker image load --input dretesteth.tar

#. Download the **dretesteth.sh** script. 

   ::

      wget https://raw.githubusercontent.com/ethereum/retesteth/master/dretesteth.sh
      chmod +x dretesteth.sh 

#. Download the tests:

   ::

      git clone --branch develop https://github.com/ethereum/tests.git

#. Run a test. This has two purposes:

   -  Create the **retesteth** configuration directories in
      **~/tests/config**, where you can modify them.
   -  A sanity check (that you can run tests successfully).

   ::

       sudo ./dretesteth.sh -t GeneralStateTests/stExample -- \
        --testpath ~/tests --datadir /tests/config 


   The output should be similar to:

   ::

      Running tests using path: /tests
      Running 1 test case...
      Retesteth config path: /tests/config
      Active client configurations: 't8ntool '
      Running tests for config 'Ethereum GO on StateTool' 2
      Test Case "stExample": (1 of 1)
      25%...
      50%...
      75%...
      100%
      
      *** No errors detected
      *** Total Tests Run: 12


   .. note:: 
       The **/tests** directory is referenced inside the docker container. It is
       the same as the **~/tests** directory outside it.

   If you get the following error:

   ::

      Tests folder does not exists, creating test folder: '/tests/GeneralStateTests/stExample'
      WARNING: /tests/src/GeneralStateTestsFiller/stExample does not exist!
      WARNING: stExample no tests detected in folder!
      Running tests for config 'Ethereum GO on StateTool' 2

      *** No errors detected
      WARNING: /tests/src/GeneralStateTestsFiller does not exist!
      *** Total Tests Run: 0

   try moving the dretesteth.sh file and cloned tests folder to your home directory (**~**).

#. To avoid having to run with **sudo** all the time, add yourself to
   the **docker** group and open a new console.

   ::

        sudo usermod -a -G docker `whoami`

