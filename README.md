<div class="oranda-hide">
# openapi-validator-proxy
</div

This project is an attempt at providing API Validation for any service that
uses an OpenAPI specification. Specifically, this project is designed to proxy
requests and responses and verify that they conform to your API specification.

## High Level Goals
- API Validation MUST be agnostic to the clients and servers.
- API Validation MUST NOT interfere with existing workflows for building APIs.
- API Validation SHOULD be easy!
	- It MUST be "one thing". You run one extra command.
    - It MUST be invoked locally the same way way as in CI.

This tool accomplishes these goals by providing the following:
- It is implemented as a proxy that does not interfere with the request or response. This makes the tool usable by any client or server combination.
- Any tests that you are currently running can be run through this proxy, effectively gaining API validation for free.
- The proxy is built as a single binary compiled for your platform. This makes it easy to run locally or in CI.
- The proxy generates a Junit report that can be used as a CI artifact.

## Installing

Check the releases page for the latest release.

### Running the Project
```
openapi-validator-proxy proxy <OPENAPI FILE> <UPSTREAM URL>

```

As in:
```
openapi-validator-proxy proxy petstore.yaml http://localhost:8080
```

This will read the OpenAPI file `petstore.yaml` and proxy requests to `http://localhost:8080`.

Including a suffix on the upstream URL is also valid if you don't mount your routes directly at the root of the server. For example, if your server is mounted at `/api/v1` you can run:
```
openapi-validator-proxy proxy petstore.yaml http://localhost:8080/api/v1
```

Then make a GET request to the pets collection like this:
```
curl http://localhost:3000/api/v1/pets
```
This will proxy the request to `http://localhost:8080/api/v1/pets` and validate the request and response against the OpenAPI operation that matches `GET /pets`.


## Contributing

### Testing

This project uses [cargo-insta](https://crates.io/crates/cargo-insta) to create snapshots of the output to test against. Insta provides a tool that makes running these tests and reviewing their output easier. To install it run `cargo install cargo-insta`. Once this is installed, changes can be reviewed with `cargo insta test --review`.

If you're just trying to run the tests you can run `cargo test`.

### Releasing this project

This project uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) and [cargo-release](https://github.com/crate-ci/cargo-release) for the release process.

The release process looks like this:
- Checkout master
- Create commit that updates RELEASES.md with notes for the new release and push commit
- Run `cargo release patch` (or minor or major) and verify the release looks correct
- Run `cargo release patch --execute --no-publish` to create the tag and push it to GitHub
- The GitHub Action should start immediately for the tag

If you are updating cargo-dist you should also run `cargo dist init` to capture changes to the action.

## Inspiration

This project was inspired by [Stoplight's Prism project](https://stoplight.io/open-source/prism), but did not make API Validation as easy as I wanted it to be.
