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
| `revm` | 6.760 s ± 0.057 |  6.691  |  6.856  |    1.00     |
| `levm` | 7.010 s ± 0.023 |  6.972  |  7.043  | 1.04 ± 0.01 |

## Fibonacci
This program computed the nth Fibonacci number, with n passed via calldata. Again, we chose 1000 as n and ran the program on a loop 100,000 times.

These are the obtained results:

### MacBook Air M1 (16 GB RAM)
|        |     Mean [s]    | Min [s] | Max [s] |  Relative   |
|--------|-----------------|---------|---------|-------------|
| `revm` | 6.257 s ± 0.093 |  6.134  |  6.422  |    1.00     |
| `levm` | 7.055 s ± 0.021 |  7.038  |  1.110  | 1.13 ± 0.02 |
