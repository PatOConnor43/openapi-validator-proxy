# Version 0.3.6 (2024-11-30)
Bug Fixes:
- Using the properties tag within a testcase doesn't seem to be well supported. Instead I've opted to use <system-out> instead to communicate what the properties for a testcase are. I believe this should be more widely supported.

# Version 0.3.5 (2024-10-28)
Bug Fixes:
- I think I messed up the JUnit template when I originally implemented it. Previously the report would start with a <testsuites> element and then start listing the <testcase>s. This release starts the report with a <testsuites> element, then a <testsuite> element, and then lists the <testcase>s. This should be the correct format.

# Version 0.3.4 (2024-10-28)
Bug Fixes:
- It turns out the proxy would panic if the response body was empty. This has been fixed by trying to get the response body bytes and if that fails, assume there wasn't a body.

# Version 0.3.3 (2024-10-27)
Chores:
  - Adding documentation pages for Custom Headers, Reports, JUnit, and Contributing
Bug Fixes:
  - Depending on if you ended the upstream suffix with a `/` or not, the proxy would not correctly match the operation in the OpenAPI file. This has been fixed so that the proxy will correctly match the operation in the OpenAPI file regardless of an ending `/` on the upstream URL.

# Version 0.3.2 (2024-10-27)
Bug Fixes:
  - Entries in the Validation Failures page were missing because the script used to generate the entries required rust nightly. The site build now uses the nightly toolchain to accomodate this.

# Version 0.3.1 (2024-10-27)
Chores:
  - Fixing the GitHub pages site. I was looking at the wrong default branch.

# Version 0.3.0 (2024-10-27)
Chores:
  - Trying to put together a GitHub pages site for the project

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

