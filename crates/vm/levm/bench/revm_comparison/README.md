# Benchmarks
This README explains how to run benchmarks to compare the performance of `levm` and `revm` when running different contracts. The benchmarking tool used to gather performance metrics is [hyperfine](https://github.com/sharkdp/hyperfine), and the obtained results will be included for reference.

To run the benchmarks (from `levm`'s root):
```bash
make revm-comparison
```

## Factorial
This program computes the nth factorial number, with n passed via calldata. We chose 1000 as n and ran the program on a loop 100,000 times.

These are the obtained results:

### MacBook Air M1 (16 GB RAM)
|        |    Mean [s]   | Min [s] | Max [s] |  Relative   |
|--------|---------------|---------|---------|-------------|
| `revm` | 6.719 ± 0.047 |  6.677  |  6.843  |    1.00     |
| `levm` | 9.497 ± 0.019 |  9.469  |  9.530  | 1.41 ± 0.01 |

## Fibonacci
This program computed the nth Fibonacci number, with n passed via calldata. Again, we chose 1000 as n and ran the program on a loop 100,000 times.

These are the obtained results:

### MacBook Air M1 (16 GB RAM)
|        |    Mean [s]   | Min [s] | Max [s] |  Relative   |
|--------|---------------|---------|---------|-------------|
| `revm` | 6.213 ± 0.029 |  6.169  |  6.253  |    1.00     |
| `levm` | 9.297 ± 0.088 |  9.214  |  9.521  | 1.50 ± 0.02 |
