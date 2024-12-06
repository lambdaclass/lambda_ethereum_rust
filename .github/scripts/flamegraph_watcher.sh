#!/bin/bash

# This script sends 1000 transactions to a test account, per defined private key
# then polls the account balance until the expected balance has been reached
# and then kills the process. It also measures the elapsed time of the test and
# outputs it to Github Action's outputs.
SECONDS=0

iterations=1000
value=10000000
account=0x33c6b73432B3aeA0C1725E415CC40D04908B85fd
end_val=$((172 * $iterations * $value))

ethrex_l2 test load --path /home/runner/work/ethrex/ethrex/test_data/private_keys.txt -i $iterations -v  --value $value --to $account

output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
while [[ $output -le $end_val ]]; do
    sleep 5
    output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
done
elapsed=$SECONDS
minutes=$((elapsed / 60))
seconds=$((elapsed % 60))
echo "Balance of $output reached in $minutes min $seconds s, killing process"

sudo pkill "$PROGRAM" && while pgrep -l "cargo-flamegraph"; do echo "waiting for $PROGRAM to exit... "; sleep 1;done;

# We need this for the following job, to add to the static page
echo "time=$minutes minutes $seconds seconds" >> "$GITHUB_OUTPUT"
