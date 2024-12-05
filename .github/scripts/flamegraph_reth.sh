#!/bin/bash

iterations=10
value=10000000
account=0x33c6b73432B3aeA0C1725E415CC40D04908B85fd
end_val=$((172 * $iterations * $value))

ethrex_l2 test load --path /home/runner/work/ethrex/ethrex/test_data/private_keys.txt -i $iterations -v  --value $value --to $account

start_time=$(date +%s)
output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
retries=0
while [[ $output -le $end_val && $retries -lt 30 ]]; do
    sleep 5
    output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
    echo "balance was $output still not reached value of $end_val (retry $retries/30)"
    ((retries++))
done
end_time=$(date +%s)
elapsed_time=$((end_time - start_time))
minutes=$((elapsed_time / 60))
seconds=$((elapsed_time % 60))
echo "Balance of $output reached in $minutes min $seconds s, killing process reth"

sudo pkill reth && while pgrep -l "cargo-flamegraph"; do echo "waiting for reth to exit... "; sleep 1;done;

# We need this for the following job, to add to the static page
echo "time=$minutes minutes $seconds seconds" >> "$GITHUB_OUTPUT"
