pragma solidity ^0.8.0;

contract Factorial {
    
    uint public result;

    constructor() {
        result = factorial(10);
    }


    function factorial(uint n) public pure returns (uint) {
        uint acc = 1;
        
        for (uint i = 2; i <= n; i++) {
            acc *= i;
        }
        
        return acc;
    }
}
