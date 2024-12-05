#!/bin/bash

iterations=3
value=10000000
account=0x33c6b73432B3aeA0C1725E415CC40D04908B85fd
end_val=$((172 * $iterations * $value))

echo "Sending to account $account"
ethrex_l2 test load --path ./test_data/private_keys.txt -i $iterations -v  --value $value --to $account

echo "Waiting for transactions to be processed..."
output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
retries=0
while [[ $output -le $end_val && $retries -lt 30 ]]; do
    sleep 5
    output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
    echo "balance was $output still not reached value of $end_val"
done
echo "Done. Balance of $output reached, killing process ethrex"
sudo pkill ethrex && while pgrep -l "cargo-flamegraph"; do echo "waiting for reth to exit... "; sleep 1;done;
echo "ethrex killed"
