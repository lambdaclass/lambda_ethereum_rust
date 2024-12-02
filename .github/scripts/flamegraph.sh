#!/bin/bash

account=0x33c6b73432B3aeA0C1725E415CC40D04908B85fd
end_val=$((172 * 1000 * 10000000))

#echo "Running ethrex..."
#CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --bin ethrex --features dev  --  --network test_data/genesis-l2.json --http.port 1729 &
#echo "Sleeping 10s before running test..."
#sleep 10
echo "Sending to account $account"
ethrex_l2 test load --path ./test_data/private_keys.txt -i 1000 -v  --value 10000000 --to $account

echo "Monitoring..."
output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
echo "ini $output"
echo "end $end_val"
while [[ $output -le $end_val ]]; do
    sleep 5
    output=$(cast balance $account --rpc-url=http://localhost:1729 2>&1)
    echo "out $output"
done
echo "Balance of $output reached, killing process ethrex"

sudo pkill ethrex && while pgrep -l cargo flamegraph; do sleep 1;done;
