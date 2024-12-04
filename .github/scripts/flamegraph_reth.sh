#!/bin/bash

iterations=3
value=10000000
account=0x33c6b73432B3aeA0C1725E415CC40D04908B85fd
end_val=$((172 * $iterations * $value))

ethrex_l2 test load --path /home/runner/work/ethrex/ethrex/test_data/private_keys.txt -i $iterations -v  --value $value --to $account

echo "Monitoring..."
output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
while [[ $output -le $end_val ]]; do
    sleep 5
    output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
done
echo "Balance of $output reached, killing process reth"

sudo pkill reth && while pgrep -l "cargo-flamegraph"; do sleep 1;done;
