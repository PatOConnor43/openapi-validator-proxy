# Version 0.2.0 (2024-10-25)
New Features:
  - Support for suffixed upstream url
    - This allows the proxy to work with servers that are mounted at a path other than the root. For example, if your server is mounted at `/api/v1` you can run `openapi-validator-proxy proxy petstore.yaml http://localhost:8080/api/v1`. Make a GET request to `http://localhost:3000/api/v1/pets` and the proxy will forward the request to `http://localhost:8080/api/v1/pets`. Validation will match against the `GET /pets` operation in the OpenAPI file.

# Version 0.1.2 (2024-10-25)
New Features:
  - Add --version flag to print the version of the binary

# Version 0.1.1 (2024-10-21)
Chores:
  - Updated README to mention looking for binaries in the releases tab
  - Added integration tests
  - Add CI workflow to run tests/linting/formatting

# Version 0.1.0 (2024-10-21)

Initial release!

The `proxy` command sort of works to validate requests. This includes functionality that validates:
- path
- HTTP method
- object response bodies
- allOf response properties
- string/boolean/number/array response properties

Results are captured in a Junit report which can be found at localhost:3000/_ovp/junit.

The following errors are also captures in the Junit report:
- PathNotFound,
- InvalidHTTPMethod,
- InvalidStatusCode,
- MissingResponseDefinition,
- MissingContentTypeHeader,
- MismatchedContentTypeHeader,
- MismatchNonEmptyBody,
- MissingSchemaDefinition,
- FailedJSONDeserialization,
- FailedValidationUnexpectedNull,
- FailedValidationUnexpectedBoolean,
- FailedValidationUnexpectedNumber,
- FailedValidationUnexpectedString,
- FailedValidationUnexpectedProperty,
- FailedValidationUnsupportedSchemaKind,

