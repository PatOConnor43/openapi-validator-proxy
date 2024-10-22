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

