# Quickstart

```
<!-- cmdrun cargo run -- --help -->
```

The main subcommand is the `proxy` command. This starts the proxy server that will validate requests and responses against the OpenAPI specification. The command takes two arguments: the path to the OpenAPI file and the URL of the upstream server. Here's an example:

```
openapi-validator-proxy proxy petstore.yaml http://localhost:8080
```

This will start the proxy server and read the OpenAPI file `petstore.yaml`. It will then proxy requests to `http://localhost:8080`. If you have a server mounted at a different path, you can include that in the URL. For example, if your server is mounted at `/api/v1` you can run:

```
openapi-validator-proxy proxy petstore.yaml http://localhost:8080/api/v1
```

Then make a GET request to the pets collection:

```http
GET http://localhost:3000/api/v1/pets
```

You can see the testcase created for this request by making an additional request to the JUnit report endpoint:

```http
GET http://localhost:3000/_ovp/junit
```

Which should return a JUnit report that resembles the following:

```xml
<testsuites tests="1" failures="0">
    <testcase name="e73ac0a9-a28e-446c-aa21-aaad827a489d" time="0.26">
        <system-out>
[[PROPERTY|correlationId=e73ac0a9-a28e-446c-aa21-aaad827a489d]]
[[PROPERTY|method=GET]]
[[PROPERTY|operationId=getPets]]
[[PROPERTY|path=/pets]]
[[PROPERTY|responseContentType=application/json]]
[[PROPERTY|statusCode=200]]
correlationId=e73ac0a9-a28e-446c-aa21-aaad827a489d
method=GET
operationId=getPets
path=/pets
responseContentType=application/json
statusCode=200
        </system-out>
    </testcase>
</testsuites>
```

The JUnit report is currently the only report but others will be added in the future.

