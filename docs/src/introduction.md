# Introduction

This project is an attempt at providing API Validation for any service that uses an OpenAPI specification. Specifically, this project is designed to proxy requests and responses and verify that they conform to your API specification.

## How it works
openapi-validator-proxy works by creating a server that listens for requests and then sends them to an upstream server. For each request it receives, it validates the response and saves the result as a testcase. Each testcase includes information about the path, method, parameters, and response code. If the response fails validation, the testcase will include an error type and message that describes the failure.


## High Level Goals
- API Validation MUST be agnostic to the clients and servers.
- API Validation MUST NOT interfere with existing workflows for building APIs.
- API Validation SHOULD be easy!
    - It MUST be "one thing". You run one extra command.
    - It MUST be invoked locally the same way as in CI.

This tool accomplishes these goals by providing the following:
- It is implemented as a proxy that does not interfere with the request or response. This makes the tool usable by any client or server combination.
- Any tests that you are currently running can be run through this proxy, effectively gaining API validation for free.
- The proxy is built as a single binary compiled for your platform. This makes it easy to run locally or in CI.
- The proxy generates a report that can be used as a CI artifact.

