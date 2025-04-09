#!/bin/bash

# This is a script that relies on rustdoc to generate a markdown table for the validation failures.
# rustdoc json output docs can be found here:
# https://rust-lang.github.io/rfcs/2963-rustdoc-json.html

cd ..
cargo +nightly rustdoc -Z unstable-options --output-format json

# This jq statement can be broken down into the following steps:
# 1. Get the index object
# 2. Convert the index object into an array of key-value pairs
# 3. Filter out the key-value pairs that are not in this crate (crate_id == 0)
# 4. Filter out the key-value pairs that are not the TestcaseFailureType enum
# 5. Convert the array of key-value pairs back into an object
# 6. Get the enum varients IDs of the TestcaseFailureType enum
# 7. Convert the enum varients IDs into a shell array
FAILURE_VARIANTS=$(jq -r '.index | to_entries | map(select(.value.crate_id == 0)) | map(select(.value.name == "TestcaseFailureType")) | from_entries | .[].inner.enum.variants | @sh' target/doc/openapi_validator_proxy.json)

# Print the table header
echo "|name|docs|"
echo "|---|---|"

# For each enum variant, print the name and docs of the variant while removing newlines and tabs
for variant in $FAILURE_VARIANTS; do
    trimmed_varient=$(echo $variant | tr -d \')
    echo -n "|"
    name=$(jq -r '.index."'"$trimmed_varient"'" | .name | gsub("[\\n\\t]"; "")' target/doc/openapi_validator_proxy.json)
    echo -n "$name"
    echo -n "|"
    docs=$(jq -r '.index."'"$trimmed_varient"'" | .docs | gsub("[\\n\\t]"; "")' target/doc/openapi_validator_proxy.json)
    echo -n "$docs"
    echo "|"
done
