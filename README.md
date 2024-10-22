# openapi-validator-proxy

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

### Release Notes
Release notes can be found on each individual tagged release as well as within the [RELEASES.md](RELEASES.md) file.

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
