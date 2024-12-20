# Custom Headers

`openapi-validator-proxy` will forward all headers it recieves from the client to the upstream server. However, there are a couple headers that have special meaning to the proxy.

## OVP-Correlation-Id

This header controls the name of the testcase when it is captured in the report. Specifying this header is completely optional. If it is not specified, the proxy will generate a UUID v4 to use as the testcase name. This can be useful if you want requests to be correlated with a specific name or identifier.

#### Example: Setting OVP-Correlation-Id

```http
GET http://localhost:3000/pets
OVP-Correlation-Id: get-pets
```

## OVP-Fused-Correlation-Headers
This header allows you to specify additional headers which should be set with the same value as the `OVP-Correlation-Id`. This is useful if you want to specify a list of headers instead of headers with redundant values, OR if you want to rely on the uuid generated by the proxy.

#### Example: Setting OVP-Correlation-Id and OVP-Fused-Correlation-Headers

```http
GET http://localhost:3000/pets
OVP-Correlation-Id: get-pets
OVP-Fused-Correlation-Headers: X-Request-Id, X-Traceid
```

This is equivalent to:

```http
GET http://localhost:3000/pets
OVP-Correlation-Id: get-pets
X-Request-Id: get-pets
X-Traceid: get-pets
```

#### Example: Fusing headers with the generated UUID

```http
GET http://localhost:3000/pets
OVP-Fused-Correlation-Headers: X-Request-Id, X-Traceid
```

When this request is received, the proxy will generate a UUID and set the headers for OVP-Correlation-Id, X-Request-Id, and X-Traceid to the same value.
