#!/bin/bash

cd ..
cargo +nightly rustdoc -Z unstable-options --output-format json
FAILURE_VARIENTS=$(jq -r '.index | to_entries | map(select(.value.crate_id == 0)) | map(select(.value.name == "TestcaseFailureType")) | from_entries | .[].inner.enum.variants | @sh' target/doc/openapi_validator_proxy.json)
echo "|name|docs|"
echo "|---|---|"
for varient in $FAILURE_VARIENTS; do
    trimmed_varient=$(echo $varient | tr -d \')
    echo -n "|"
    name=$(jq -r '.index."'"$trimmed_varient"'" | .name | gsub("[\\n\\t]"; "")' target/doc/openapi_validator_proxy.json)
    echo -n "$name"
    echo -n "|"
    docs=$(jq -r '.index."'"$trimmed_varient"'" | .docs | gsub("[\\n\\t]"; "")' target/doc/openapi_validator_proxy.json)
    echo -n "$docs"
    echo "|"

done
