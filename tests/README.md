## Integration tests

The tests in this folder are integration tests. Concretely, that means that the way these tests work are by actually running the openapi-valdator-proxy binary and sending requests through it. These tests are made of 3 main parts:
- the proxy binary
- a mock server that the proxy will forward requests to
- a "snapshot" of the resulting junit report from the proxy

You can see the existing snapshots within the `snapshots` folder. In order to add more, write the test as you normally would and include a call to `insta::assert_snapshot!(...)`. This will stage a new snapshot that you can choose to accept or reject with `cargo insta test --review`.
