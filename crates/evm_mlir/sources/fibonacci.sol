pragma solidity ^0.8.0;

contract Fibonacci {

    uint public result;

    constructor() {
        result = fibonacci(10);
    }


    function fibonacci(uint n) public pure returns (uint) {

        if (n == 0) {
            return 0;
        } else if (n == 1) {
            return 1;
        }

        uint a = 0;
        uint b = 1;
        uint acc;

        for (uint i = 2; i <= n; i++) {
            acc = a + b;
            a = b;
            b = acc;
        }

        return acc;
    }
}
