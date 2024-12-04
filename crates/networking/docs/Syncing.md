# Syncing

## Snap Sync

A snap sync cycle begins by fetching all the block headers (via p2p) between the current head (latest canonical block) and the sync head (block hash sent by a forkChoiceUpdate).
The next two steps are performed in parallel:
On one side, blocks and receipts for all fetched headers are fetched via p2p and stored.
On the other side, the state is reconstructed via p2p snap requests. Our current implementation of this works as follows:
We will spawn two processes, the `bytecode_fetcher` which will remain active and process bytecode requests in batches by requesting bytecode batches from peers and storing them, and the `fetch_snap_state` process, which will iterate over the fetched headers and rebuild the block's state via `rebuild_state_trie`.
`rebuild_state_trie` will span a `storage_fetcher` process (which works similarly to the `bytecode_fetcher` and is kept alive for the duration of the rebuild process), it will open a new state trie and will fetch the block's accounts in batches and for each account it will: send the accoun's code hash to the `bytecode_fetcher` (if not empty), send the account's address and storage root to the `storage_fetcher` (if not empty), and add the account to the state trie. Once all accounts are processed, the final state root will be checked and commited.
(Not implemented yet) When `fetch_snap_state` runs out of available state (aka, the state we need to fetch is older than 128 blocks and peers don't provide it), it will begin the `state_healing` process.
This diagram illustrates the process described above:
![snap_sync](/crates/networking/docs/diagrams/snap_sync.jpg)
