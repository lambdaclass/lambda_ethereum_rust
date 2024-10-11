# Common

This crate contains common utilities and data structures that are used across the client.

## RLP

Recursive Length Prefix (RLP) is used as the main serialization format in Ethereum. It is used both to store data on disk and to encode messages that are sent between nodes in the network.

More information (here)(https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/)

The main traits that need to be implemented are `RLPEncode` and `RLPDecode`, which can be implemented using the `Encoder` and `Decoder` structs.
