pragma solidity ^0.8.0;

contract RecursiveFibonacci {

    uint public result;

    constructor() {
        result = fibonacci(10);
    }

    function fibonacci(uint n) public pure returns (uint) {
        if (n == 0) {
            return 0;
        } else if (n == 1) {
            return 1;
        } else {
            return fibonacci(n - 1) + fibonacci(n - 2);
        }
    }
}
