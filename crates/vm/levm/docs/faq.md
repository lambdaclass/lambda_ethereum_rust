# FAQ
## `usize` and U256
In Rust, accessing an index on a specific data structure requires a `usize` type variable. This can be seen in methods like `get` and `get_mut`.

<!-- TODO: Link in the documentation where the U256 addresses are described -->
On the other hand, the EVM specification requires all addresses to be in U256. Therefore, every opcode treats its arguments as U256 values.
The problem arises in the opcodes that need to access a specific index on a data structure (e.g. `CALLDATA`, `CODECOPY`, `EXTCODECOPY`, etc).
These operands receive offsets and indexes in U256, but the data structure they have to access (e.g. `Memory` or  `Calldata`) require a `usize`. Therefore, those parameters need to be cast from U256 to `usize`.
The problem is, U256's representation ranger is larger than `usize`'s; so not all numbers can be successfully cast. In these cases, special attention is needed.

The main way to deal with theses cases (at least, at the time of writing) is to **delay the cast**. Before casting to `usize`, we compare the size of the index in U256 with the length of the data structure it wants to access. Here's an example from the `EXTCODECOPY` opcode (NOTE: the code snippet is a simplified/altered version to demonstrate this pattern):

Some context: It is not important what this operand does. The only thing that matters for this example is that `EXTCODECOPY` returns a vector of bytes. That vector will copy a specific amount of bytes from `calldata` to `memory`. 
Notably for this example, it can receive an offset. Which will tell the operand which parts of the `calldata` section it should skip. Skipped sections will be replaced with 0's.
```rust
0:   pub fn op_extcodecopy(

        (...)

1:        let offset: U256 = current_call_frame.stack.pop()?; // This represents a `calldata` offset.

        (...)

2:       let mut data = vec![0u8; size];
3:       if offset < account_info.bytecode.len().into() {
4:           let offset: usize = offset
5:               .try_into()
6:               .map_err(|_| VMError::Internal(InternalError::ConversionError))?;
7:           // After this the data vector is modified

        (...)
8:      }
9:      memory::try_store_data(&mut current_call_frame.memory, dest_offset, &data)?;

10:     Ok(OpcodeSuccess::Continue)
```

- In line `1` we get the offset, which is in U256.
- In line `2` we create the vector which we will return.
- In line `3` we check if the calldata offset is larger than the calldata itself. If this is the case, there's no data to copy. So we do not want to modify the vector.
    -  If it is not larger, we can safely cast it to usize (which is done in line `4`). This is because the `calldata` size is capped <!-- TODO: Add link to where this is specified. -->
- Finally in line `9`, we store the data vector in memory.
    - As stated previously, this can be a vector of all 0's if the calldata offset was larger than calldata itself.


This pattern is fairly common and is useful to keep in mind when dealing with operands that deal with offsets and indexes.
