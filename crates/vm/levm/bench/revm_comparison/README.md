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
|        |     Mean [s]    | Min [s] | Max [s] |  Relative   |
|--------|-----------------|---------|---------|-------------|
| `revm` | 6.726 s ± 0.052 |  6.661  |  6.807  |    1.00     |
| `levm` | 7.508 s ± 0.087 |  7.441  |  7.744  | 1.12 ± 0.02 |

## Fibonacci
This program computed the nth Fibonacci number, with n passed via calldata. Again, we chose 1000 as n and ran the program on a loop 100,000 times.

These are the obtained results:

### MacBook Air M1 (16 GB RAM)
|        |     Mean [s]    | Min [s] | Max [s] |  Relative   |
|--------|-----------------|---------|---------|-------------|
| `revm` | 6.271 s ± 0.026 |  6.234  |  6.312  |    1.00     |
| `levm` | 7.329 s ± 0.078 |  7.274  |  7.532  | 1.17 ± 0.01 |
