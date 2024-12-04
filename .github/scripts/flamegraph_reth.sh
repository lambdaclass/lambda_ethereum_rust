#!/bin/bash
cd "$(dirname "$0")" || exit # Make sure to run from script's directory

account=0x33c6b73432B3aeA0C1725E415CC40D04908B85fd
value=10000000
end_val=$((172 * 1000 * $value))

ethrex_l2 test load --path ../../test_data/private_keys.txt -i 1000 -v  --value $value --to $account

echo "Monitoring..."
output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
echo "ini $output"
echo "end $end_val"
while [[ $output -le $end_val ]]; do
    sleep 5
    output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
    echo "out $output"
done
echo "Balance of $output reached, killing process reth"

sudo pkill reth && while pgrep -l cargo flamegraph; do sleep 1;done;
