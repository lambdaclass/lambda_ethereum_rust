# Lambda Ethereum Rust Execution Client

[![Telegram Chat][tg-badge]][tg-url]
[![license](https://img.shields.io/github/license/lambdaclass/ethereum_rust)](/LICENSE)

[tg-badge]: https://img.shields.io/endpoint?url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Frust_ethereum%2F&logo=telegram&label=chat&color=neon
[tg-url]: https://t.me/rust_ethereum

## Philosophy

Many long-established clients accumulate bloat over time. This often occurs due to the need to support legacy features for existing users or through attempts to implement overly ambitious software. The result is often complex, difficult-to-maintain, and error-prone systems.

In contrast, our philosophy is rooted in simplicity. We strive to write minimal code, prioritize clarity, and embrace simplicity in design. We believe this approach is the best way to build a client that is both fast and resilient. By adhering to these principles, we will be able to iterate fast and explore next-generation features early, either from the Ethereum roadmap or from innovations from the L2s.

Read more about our engineering philosophy [here](https://blog.lambdaclass.com/lambdas-engineering-philosophy/)

## Design Principles

- Ensure effortless setup and execution across all target environments.
- Be vertically integrated. Have the minimal amount of dependencies.
- Be structured in a way that makes it easy to build on top of it, i.e rollups, vms, etc.
- Have a simple type system. Avoid having generics leaking all over the codebase.
- Have few abstractions. Do not generalize until you absolutely need it. Repeating code two or three times can be fine.
- Prioritize code readability and maintainability over premature optimizations.
- Avoid concurrency split all over the codebase. Concurrency adds complexity. Only use where strictly necessary.

## Quick Start (localnet)

![Demo](https://raw.githubusercontent.com/lambdaclass/ethereum_rust/8e3b69d727225686eec30b2c2b79cecdf7eac2d9/Demo.png)

### Prerequisites
- [Kurtosis](https://docs.kurtosis.com/install/#ii-install-the-cli)
- [Rust](#rust)
- [Docker](https://docs.docker.com/engine/install/)
```shell
make localnet
```

This make target will:
1. Build our node inside a docker image.
2. Fetch our fork [ethereum package](https://github.com/ethpandaops/ethereum-package), a private testnet on which multiple ethereum clients can interact.
3. Start the localnet with kurtosis.

If everything went well, you should be faced with our client's logs (ctrl-c to leave)

To stop everything, simply run:
```shell
make stop-localnet
```

## Dev Setup
### Build

#### Rust
To build the node, you will need the rust toolchain:
1. First, [install asdf](https://asdf-vm.com/guide/getting-started.html):
2. Add the rust plugin:
```shell
asdf plugin-add rust https://github.com/asdf-community/asdf-rust.git
```
3. cd into the project and run:
```shell
asdf install
```

You now should be able to build the client:
```bash
make build
```
### Database
Currently, the database is `libmdbx`, it will be set up
when you start the client. The location of the db's files will depend on your OS:
- Mac: `~/Library/Application Support/ethereum_rust`
- Linux: `~/.config/ethereum_rust`

You can delete the db with:
```bash
cargo run --bin ethereum_rust -- removedb
```

### Test

For testing, we're using three kinds of tests.

##### Ethereum Foundation Tests

These are the official execution spec tests, you can execute them with:

```bash
make test
```

This will download the test cases from the [official execution spec tests repo](https://github.com/ethereum/execution-spec-tests/) and run them with our glue code
under `cmd/ef_tests/tests`.

##### Crate Specific Tests

The second kind are each crate's tests, you can run them like this:

```bash
make test CRATE=<crate>
```
For example:
```bash
make test CRATE="ethereum_rust-blockchain"
```


##### Hive Tests

Finally, we have End-to-End tests with hive.
Hive is a system which simply sends RPC commands to our node,
and expects a certain response. You can read more about it [here](https://github.com/ethereum/hive/blob/master/docs/overview.md).
Hive tests are categorized by "simulations', and test instances can be filtered with a regex:
```bash
make run-hive-debug SIMULATION=<simulation> TEST_PATTERN=<test-regex>
```
This is an example of a Hive simulation called `ethereum/rpc-compat`, which will specificaly
run chain id and transaction by hash rpc tests:
```bash
make run-hive SIMULATION=ethereum/rpc-compat TEST_PATTERN="/eth_chainId|eth_getTransactionByHash"
```
If you want debug output from hive, use the run-hive-debug instead:
```bash
make run-hive-debug SIMULATION=ethereum/rpc-compat TEST_PATTERN="*"
```
This example runs **every** test under rpc, with debug output

### Run

Example run:
```bash
cargo run --bin ethereum_rust -- --network test_data/genesis-kurtosis.json
```

The `network` argument is mandatory, as it defines the parameters of the chain.
For more information about the different cli arguments check out the next section.

### CLI Commands

Ethereum Rust supports the following command line arguments:
- `--network <FILE>`: Receives a `Genesis` struct in json format. This is the only argument which is required. You can look at some example genesis files at `test_data/genesis*`.
- `--datadir <DIRECTORY>`: Receives the name of the directory where the Database is located.
- `--import <FILE>`: Receives an rlp encoded `Chain` object (aka a list of `Block`s). You can look at the example chain file at `test_data/chain.rlp`.
- `--http.addr <ADDRESS>`: Listening address for the http rpc server. Default value: localhost.
- `--http.port <PORT>`: Listening port for the http rpc server. Default value: 8545.
- `--authrpc.addr <ADDRESS>`: Listening address for the authenticated rpc server. Default value: localhost.
- `--authrpc.port <PORT>`: Listening port for the authenticated rpc server. Default value: 8551.
- `--authrpc.jwtsecret <FILE>`: Receives the jwt secret used for authenticated rpc requests. Default value: jwt.hex.
- `--p2p.addr <ADDRESS>`: Default value: 0.0.0.0.
- `--p2p.port <PORT>`: Default value: 30303.
- `--discovery.addr <ADDRESS>`: UDP address for P2P discovery. Default value: 0.0.0.0.
- `--discovery.port <PORT>`: UDP port for P2P discovery. Default value: 30303.
- `--bootnodes <BOOTNODE_LIST>`: Comma separated enode URLs for P2P discovery bootstrap.

## Roadmap

### Milestone 1: RPC Node

Add support to follow a post-Merge localnet as a read-only RPC Node. This first milestone will only support a canonical chain (every incoming block has to be the child of the current head).

RPC endpoints

- `debug_getRawBlock` ✅
- `debug_getRawHeader` ✅
- `debug_getRawReceipts` ✅
- `debug_getRawTransaction` ✅
- `engine_exchangeCapabilities`
- `engine_exchangeTransitionConfiguration` ✅
- `engine_newPayload` ✅
- `eth_blobBaseFee` ✅
- `eth_blockNumber` ✅
- `eth_call` (at head block) ✅
- `eth_chainId` ✅
- `eth_createAccessList` (at head block) ✅
- `eth_estimateGas` ✅
- `eth_feeHistory`
- `eth_getBalance` (at head block) ✅
- `eth_getBlockByHash` ✅
- `eth_getBlockByNumber` ✅
- `eth_getBlockReceipts` ✅
- `eth_getBlockTransactionCountByNumber` ✅
- `eth_getCode` (at head block) ✅
- `eth_getFilterChanges`
- `eth_getFilterLogs`
- `eth_getLogs`
- `eth_getStorageAt` (at head block) ✅
- `eth_getTransactionByBlockHashAndIndex` ✅
- `eth_getTransactionByBlockNumberAndIndex` ✅
- `eth_getTransactionByHash` ✅
- `eth_getTransactionCount` ✅
- `eth_newBlockFilter`
- `eth_newFilter`
- `eth_newPendingTransactionFilter`
- `eth_uninstallFilter`

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/1>

### Milestone 2: History & Reorgs
Implement support for block reorganizations and historical state queries. This milestone involves persisting the state trie to enable efficient access to historical states and implementing a tree structure for the blockchain to manage multiple chain branches.

RPC endpoints
- `engine_forkchoiceUpdated` (without `payloadAttributes`)
- `eth_call` (at any block) ✅
- `eth_createAccessList` (at any block) ✅
- `eth_getBalance` (at any block) ✅
- `eth_getCode` (at any block) ✅
- `eth_getProof` ✅
- `eth_getStorageAt` (at any block) ✅

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/4

### Milestone 3: Block building
Add the ability to build new payloads, so that the consensus client can propose new blocks based on transactions received from the RPC endpoints.

RPC endpoints
- `engine_forkchoiceUpdated` (with `payloadAttributes`)
- `engine_getPayload`
- `eth_sendRawTransaction` ✅

### Milestone 4: P2P Network
Implement DevP2P protocol, including RLPx `p2p` and `eth` features. This will let us get and send blocks and transactions from other nodes. We'll add the transactions we receive to the mempool. We'll also download blocks from other nodes when we get payloads where the parent isn't in our local chain.

RPC endpoints

- `admin_nodeInfo` ✅

See issues and progress: <https://github.com/lambdaclass/ethereum_rust/milestone/2>

### Milestone 5: Syncing
Add snap sync protocol, which lets us get a recent copy of the blockchain state instead of going through all blocks from genesis. Since we don't support older versions of the spec by design, this is a prerequisite to being able to sync the node with public networks, including mainnet.

RPC endpoints
- `eth_syncing`

See issues and progress: https://github.com/lambdaclass/ethereum_rust/milestone/3

# Crates

In the next sections, you can dive further into the code internals.

-   [net](#network)

## Network

The network crate handles the ethereum networking protocols. This involves:

-   [Discovery protocol](#discovery-protocol): built on top of udp and it is how we discover new nodes.
-   devP2P: sits on top of tcp and is where the actual blockchain information exchange happens.

The official spec can be found [here](https://github.com/ethereum/devp2p/tree/master).

### Discovery protocol

In the next section, we'll be looking at the discovery protocol (discv4 to be more specific) and the way we have it set up. There are many points for improvement and here we discuss some possible solutions to them.

At startup, the discovery server launches three concurrent tokio tasks:

-   The listen loop for incoming requests.
-   A revalidation loop to ensure peers remain responsive.
-   A recursive lookup loop to request new peers and keep our table filled.

Before starting these tasks, we run a [startup](#startup) process to connect to an array of initial nodes.

Before diving into what each task does, first, we need to understand how we are storing our nodes. Nodes are stored in an in-memory matrix which we call a [Kademlia table](https://github.com/lambdaclass/ethereum_rust/blob/main/crates/net/kademlia.rs#L20-L23), though it isn't really a Kademlia table as we don't thoroughly follow the spec but we take it as a reference, you can read more [here](https://en.wikipedia.org/wiki/Kademlia). This table holds:

-   Our `node_id`: `node_id`s are derived from the public key. They are the 64 bytes starting from index 1 of the encoded pub key.
-   A vector of 256 `bucket`s which holds:
    -   `peers`: a vector of 16 elements of type `PeersData` where we save the node record and other related data that we'll see later.
    -   `replacements`: a vector of 16 elements of `PeersData` that are not connected to us, but we consider them as potential replacements for those nodes that have disconnected from us.

Peers are not assigned to any bucket but they are assigned based on its $0 \le \text{distance} \le 255$ to our `node_id`. Distance is defined by:

```rust
pub fn distance(node_id_1: H512, node_id_2: H512) -> usize {
    let hash_1 = Keccak256::digest(node_id_1);
    let hash_2 = Keccak256::digest(node_id_2);
    let xor = H256(hash_1.into()) ^ H256(hash_2.into());
    let distance = U256::from_big_endian(xor.as_bytes());
    distance.bits().saturating_sub(1)
}
```

#### Startup

Before starting the server, we do a startup where we connect to an array of seeders or bootnodes. This involves:

-   Receiving bootnodes via CLI params
-   Inserting them into our table
-   Pinging them to notify our presence, so they acknowledge us.

This startup is far from being completed. The current state allows us to do basic tests and connections. Later, we want to do a real startup by first trying to connect to those nodes we were previously connected. For that, we'd need to store nodes on the database. If those nodes aren't enough to fill our table, then we also ping some bootnodes, which could be hardcoded or received through the cli. Current issues are opened regarding [startup](https://github.com/lambdaclass/ethereum_rust/issues/398) and [nodes db](https://github.com/lambdaclass/ethereum_rust/issues/454).

#### Listen loop

The listen loop handles messages sent to our socket. The spec defines 6 types of messages:

-   **Ping**: Responds with a `pong` message. If the peer is not in our table we add it, if the corresponding bucket is already filled then we add it as a replacement for that bucket. If it was inserted we send a `ping from our end to get an endpoint proof.
-   **Pong**: Verifies that the `pong` corresponds to a previously sent `ping`, if so we mark the peer as proven.
-   **FindNodes**: Responds with a `neighbors` message that contains as many as the 16 closest nodes from the given target. A target is a pubkey provided by the peer in the message. The response can't be sent in one packet as it might exceed the discv4 max packet size. So we split it into different packets.
-   **Neighbors**: First we verify that we have sent the corresponding `find_node` message. If so, we receive the peers, store them, and ping them. Also, every [`find_node` request](https://github.com/lambdaclass/ethereum_rust/blob/229ca0b316a79403412a917d04e3b95f579c56c7/crates/net/discv4.rs#L305-L314) may have a [tokio `Sender`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html) attached, if that is the case, we forward the nodes from the message through the channel. This becomes useful when waiting for a `find_node` response, [something we do in the lookups](https://github.com/lambdaclass/ethereum_rust/blob/229ca0b316a79403412a917d04e3b95f579c56c7/crates/net/net.rs#L517-L570).
-   **ENRRequest**: currently not implemented see [here](https://github.com/lambdaclass/ethereum_rust/issues/432).
-   **ENRResponse**: same as above.

#### Re-validations

Re-validations are tasks that are implemented as intervals, that is: they run an action every `x` wherever unit of time (currently configured to run every 30 seconds). The current flow of re-validation is as follows

1. Every 30 seconds (by default) we ping the three least recently pinged peers: this may be fine now to keep simplicity, but we might prefer to choose three random peers instead to avoid the search which might become expensive as our buckets start to fill with more peers.
2. In the next iteration we check if they have answered
    - if they have: we increment the liveness field by one.
    - otherwise: we decrement the liveness by a third of its value.
3. If the liveness field is 0, we delete it and insert a new one from the replacements table.

Liveness is a field that provides us with good criteria of which nodes are connected and we "trust" more. This trustiness is useful when deciding if we want to store this node in the database to use it as a future seeder or when establishing a connection in p2p.

Re-validations are another point of potential improvement. While it may be fine for now to keep simplicity at max, pinging the last recently pinged peers becomes quite expensive as the number of peers in the table increases. And it also isn't very "just" in selecting nodes so that they get their liveness increased so we trust them more and we might consider them as a seeder. A possible improvement could be:

-   Keep two lists: one for nodes that have already been pinged, and another one for nodes that have not yet been revalidated. Let's call the former "a" and the second "b".
-   In the beginning, all nodes would belong to "a" and whenever we insert a new node, they would be pushed to "a".
-   We would have two intervals: one for pinging "a" and another for pinging to nodes in "b". The "b" would be quicker, as no initial validation has been done.
-   When picking a node to ping, we would do it randomly, which is the best form of justice for a node to become trusted by us.
-   When a node from `b` responds successfully, we move it to `a`, and when one from `a` does not respond, we move it to `b`.

#### Recursive Lookups

Recursive lookups are as with re-validations implemented as intervals. Their current flow is as follows:

1. Every 30min we spawn three concurrent lookups: one closest to our pubkey and three others closest to randomly generated pubkeys.
2. Every lookup starts with the closest nodes from our table. Each lookup keeps track of:
    - Peers that have already been asked for nodes
    - Peers that have been already seen
    - Potential peers to query for nodes: a vector of up to 16 entries holding the closest peers to the pubkey. This vector is initially filled with nodes from our table.
3. We send a `find_node` to the closest 3 nodes (that we have not yet asked) from the pubkey.
4. We wait for the neighbors' response and push or replace those who are closer to the potential peers.
5. We select three other nodes from the potential peers vector and do the same until one lookup has no node to ask.

#### An example of how you might build a network

Finally, here is an example of how you could build a network and see how they connect each other:

We'll have three nodes: `a`, `b`, and `c`, we'll start `a`, then `b` setting `a` as a bootnode, and finally we'll start `c` with `b` as bootnode we should see that `c` connects to both `a` and `b` and so all the network should be connected.

**node a**:
`cargo run --bin ethereum_rust --network test_data/kurtosis.json`

We get the `enode` by querying the node_info:
`curl http://localhost:8545 \
  -X POST \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"admin_nodeInfo","params":[],"id":1}'`

**node b**
We start a new server passing the `node_a` `enode` as bootnodes

```bash
cargo run --bin ethereum_rust --network ./test_data/kurtosis.json --bootnodes=`NODE_A_ENODE` \
--authrpc.port=8552 --http.port=8546 --p2p.port=30305 --discovery.port=3036
```

**node c**
Finally, with `node_c` we connect to `node_b`. When the lookup runs, `node_c` should end up connecting to `node_a`:

```bash
 cargo run --bin ethereum_rust --network ./test_data/kurtosis.json --bootnodes=`NODE_B_ENODE`" \
--authrpc.port=8553 --http.port=8547 --p2p.port=30308 --discovery.port=30310
```

You could also spawn nodes from other clients and it should work as well.
