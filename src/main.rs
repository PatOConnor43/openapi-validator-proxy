use askama::Template;
use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue},
    response::IntoResponse,
    routing::{delete, get, head, options, patch, post, put},
    Router,
};
use axum_macros::debug_handler;
use clap::{Parser, Subcommand};
use openapiv3::ReferenceOr;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use tokio::{signal, sync::Mutex};
use tracing::{debug, error, info, instrument, Level};
use tracing_subscriber::FmtSubscriber;
use ureq::OrAnyStatus;

#[derive(Parser)]
#[command(
    about = "A CLI application to validate OpenAPI specification requests and responses.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Starts the proxy server with the given file as input
    Proxy {
        /// Filepath of the OpenAPI spec
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Filepath of the OpenAPI spec
        #[arg(value_name = "UPSTREAM")]
        upstream: url::Url,

        #[arg(short, long)]
        port: Option<u16>,
    },
}

#[derive(Clone)]
struct AppState {
    spec: openapiv3::OpenAPI,
    upstream: url::Url,
    testcases: Arc<Mutex<Vec<Testcase>>>,
    wayfinder: wayfind::Router<()>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("spec", &self.spec)
            .field("upstream", &self.upstream)
            .field("testcases", &self.testcases)
            .field("wayfinder", &"wayfinder::Router<()>")
            .finish()
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "junit.xml")]
struct JunitTemplate {
    testcases: Vec<Testcase>,
    failed_testcases: usize,
}

#[derive(Debug, Clone)]
struct Testcase {
    name: String,
    failures: Vec<TestcaseFailure>,
    properties: Vec<TestcaseProperty>,
    time: String,
}

#[derive(Debug, Clone)]
struct TestcaseProperty {
    name: String,
    value: String,
}

#[derive(Debug, Clone)]
struct TestcaseFailure {
    text: String,
    r#type: TestcaseFailureType,
}

/// An enum describing the type of test failure that occurred.
#[derive(Debug, Clone)]
enum TestcaseFailureType {
    /// The requested path was not found in the OpenAPI spec. This response was not validated
    /// and may be missing relevant testcase properties.
    PathNotFound,
    /// The HTTP method used in the request is not one of the expected values:
    /// DELETE, GET, HEAD, OPTIONS, PATCH, POST, PUT, or TRACE.
    InvalidHTTPMethod,
    /// The status code returned by the upstream server does not have a matching response in the OpenAPI spec.
    InvalidStatusCode,
    /// The OpenAPI spec contained a missing inline response definition or referenced a response that did not exist.
    MissingResponseDefinition,
    /// The upstream server did not include a Content-Type header in the response. This is only an
    /// issue when the response body is not empty.
    MissingContentTypeHeader,
    /// The upstream server included a Content-Type header in the response that does not match any
    /// content types defined in the OpenAPI spec.
    MismatchedContentTypeHeader,
    /// The upstream server included a non-empty response body when the OpenAPI spec expects an empty body.
    MismatchNonEmptyBody,
    /// The OpenAPI spec contained a missing inline schema definition or referenced a schema that did not exist.
    MissingSchemaDefinition,
    /// The response body could not be deserialized as JSON.
    FailedJSONDeserialization,
    /// The response body contains a null value when the OpenAPI spec did not allow null values.
    FailedValidationUnexpectedNull,
    /// The response body contained a boolean value when the OpenAPI spec expected a different type.
    FailedValidationUnexpectedBoolean,
    /// The response body contained a number value when the OpenAPI spec expected a different type.
    FailedValidationUnexpectedNumber,
    /// The response body contained a string value when the OpenAPI spec expected a different type.
    FailedValidationUnexpectedString,
    /// The response body contained a property that was not defined in the OpenAPI spec.
    FailedValidationUnexpectedProperty,
    /// The OpenAPI spec contained a schema with an unsupported kind, such as anyOf, oneOf, or not.
    FailedValidationUnsupportedSchemaKind,
}

impl std::fmt::Display for TestcaseFailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TestcaseFailureType::PathNotFound => write!(f, "PathNotFound"),
            TestcaseFailureType::InvalidHTTPMethod => write!(f, "InvalidHTTPMethod"),
            TestcaseFailureType::InvalidStatusCode => write!(f, "InvalidStatusCode"),
            TestcaseFailureType::MissingResponseDefinition => {
                write!(f, "MissingResponseDefinition")
            }
            TestcaseFailureType::MissingContentTypeHeader => {
                write!(f, "MissingContentTypeHeader")
            }
            TestcaseFailureType::MismatchedContentTypeHeader => {
                write!(f, "MismatchedContentTypeHeader")
            }
            TestcaseFailureType::MismatchNonEmptyBody => write!(f, "MismatchNonEmptyBody"),
            TestcaseFailureType::MissingSchemaDefinition => write!(f, "MissingSchemaDefinition"),
            TestcaseFailureType::FailedJSONDeserialization => {
                write!(f, "FailedJSONDeserialization")
            }
            TestcaseFailureType::FailedValidationUnexpectedNull => {
                write!(f, "FailedValidation.UnexpectedNull")
            }
            TestcaseFailureType::FailedValidationUnexpectedBoolean => {
                write!(f, "FailedValidation.UnexpectedBoolean")
            }
            TestcaseFailureType::FailedValidationUnexpectedNumber => {
                write!(f, "FailedValidation.UnexpectedNumber")
            }
            TestcaseFailureType::FailedValidationUnexpectedString => {
                write!(f, "FailedValidation.UnexpectedString")
            }
            TestcaseFailureType::FailedValidationUnexpectedProperty => {
                write!(f, "FailedValidation.UnexpectedProperty")
            }
            TestcaseFailureType::FailedValidationUnsupportedSchemaKind => {
                write!(f, "FailedValidation.UnsupportedSchemaKind")
            }
        }
    }
}

struct ValidatedResponse {
    body: Vec<u8>,
    failures: Vec<TestcaseFailure>,
    headers: axum::http::HeaderMap,
    #[allow(dead_code)]
    method: axum::http::Method,
    properties: Vec<TestcaseProperty>,
    status: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Proxy {
            file,
            upstream,
            port,
        } => {
            println!(
                "Starting proxy server with file: {:?}, upstream: {}",
                file, upstream
            );
            let metadata = std::fs::metadata(file)?;
            if metadata.is_file() {
                let content = std::fs::read_to_string(file)?;
                let spec = parse_openapi_spec(&content)?;
                start_server(spec, upstream.clone(), port.unwrap_or(3000)).await;
            } else {
                return Err(format!("Error: {:?} is not a file", file).into());
            }
        }
    }
    Ok(())
}

fn parse_openapi_spec(content: &str) -> Result<openapiv3::OpenAPI, Box<dyn std::error::Error>> {
    if content.starts_with("{") {
        let spec: openapiv3::OpenAPI = serde_json::from_str(content)?;
        Ok(spec)
    } else {
        let spec: openapiv3::OpenAPI = serde_yaml::from_str(content)?;
        Ok(spec)
    }
}

async fn start_server(spec: openapiv3::OpenAPI, upstream: url::Url, port: u16) {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut wayfinder = wayfind::Router::new();
    for (path_template, _) in spec.paths.paths.iter() {
        let path_template = path_template.to_string();
        wayfinder.insert(&path_template, ()).unwrap();
    }

    let state = AppState {
        spec,
        upstream,
        testcases: Arc::new(Mutex::new(vec![])),
        wayfinder,
    };

    let app = Router::new()
        .route("/_ovp/junit", get(junit))
        .route("/*path", delete(root))
        .route("/*path", get(root))
        .route("/*path", head(root))
        .route("/*path", options(root))
        .route("/*path", patch(root))
        .route("/*path", post(root))
        .route("/*path", put(root))
        .with_state(state);

    // Run the Axum server
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

#[instrument(skip_all)]
#[debug_handler(state = AppState)]
async fn junit(state: State<AppState>) -> impl IntoResponse {
    let testcases = state.testcases.lock().await.clone();
    let testcases_with_failures = testcases
        .iter()
        .filter(|testcase| !testcase.failures.is_empty())
        .count() as usize;
    let template = JunitTemplate {
        testcases,
        failed_testcases: testcases_with_failures,
    };
    let rendered = template.render().unwrap();
    let mut header_map = axum::http::HeaderMap::new();
    header_map.insert("Content-Type", HeaderValue::from_static("application/xml"));

    (axum::http::StatusCode::OK, header_map, rendered)
}

#[instrument(skip_all)]
#[debug_handler(state = AppState)]
async fn root(state: State<AppState>, request: Request) -> impl IntoResponse {
    inner_handler(state, request).await
}

async fn inner_handler(
    State(AppState {
        spec,
        upstream,
        testcases,
        wayfinder,
    }): State<AppState>,
    request: Request,
) -> impl IntoResponse {
    let mut failures = vec![];
    let mut properties = vec![];
    let method = request.method().clone();
    let path = request.uri().path();
    let path_and_query = request.uri().path_and_query().unwrap();
    let url = upstream.join(path_and_query.as_str()).unwrap();
    info!("Handling request: {} {}", method, url);
    properties.push(TestcaseProperty {
        name: "path".to_string(),
        value: path.to_string(),
    });
    properties.push(TestcaseProperty {
        name: "method".to_string(),
        value: method.to_string(),
    });

    let wayfinder_path = wayfind::Path::new(path).unwrap();
    let wayfinder_match = wayfinder.search(&wayfinder_path).unwrap();
    match &wayfinder_match {
        Some(wayfound) => {
            for parameter in wayfound.parameters.iter() {
                properties.push(TestcaseProperty {
                    name: format!("pathParameter-{}", parameter.key),
                    value: parameter.value.to_string(),
                });
            }
        }
        None => {
            failures.push(TestcaseFailure {
                text: "Path not found".to_string(),
                r#type: TestcaseFailureType::PathNotFound,
            });
        }
    }
    let wayfinder_path = wayfinder_match.map(|m| m.route.to_string());

    let mut outgoing_request = ureq::request(method.as_str(), url.as_str());
    for (key, value) in request.headers() {
        let key = key.as_str();
        let value = value.to_str().unwrap();
        outgoing_request = outgoing_request.set(key, value);
    }
    // The correlation ID is what is used to specify the name of the testcase. If the client
    // supplied one, use that. Otherwise, generate a new one.
    let correlation_id = match outgoing_request.header("OVP-Correlation-Id") {
        Some(correlation_id) => correlation_id.to_string(),
        None => {
            let generated_uuid = uuid::Uuid::new_v4().to_string();
            outgoing_request = outgoing_request.set("OVP-Correlation-Id", &generated_uuid);
            generated_uuid
        }
    };
    // If the client supplied a list of headers to fuse, add them to the outgoing request
    if let Some(fuse_headers) = outgoing_request.header("OVP-Fused-Correlation-Headers") {
        let fuse_headers = fuse_headers.to_string();
        for header in fuse_headers.split(",") {
            let header = header.trim();
            if header.is_empty() {
                continue;
            }
            outgoing_request = outgoing_request.set(header, &correlation_id);
        }
    }

    let body = axum::body::to_bytes(request.into_body(), usize::MAX)
        .await
        .unwrap();
    let time_start = std::time::Instant::now();
    let response = outgoing_request.send_bytes(&body).or_any_status().unwrap();
    let time_end = std::time::Instant::now();
    let duration = time_end - time_start;
    let mut validated_response = validate_response(response, method, &spec, wayfinder_path);
    failures.append(&mut validated_response.failures);
    properties.append(&mut validated_response.properties);
    let mut cases = testcases.lock().await;
    cases.push(Testcase {
        name: correlation_id.to_string(),
        failures,
        properties,
        time: format!("{:.2}", duration.as_secs_f64()),
    });
    drop(cases);
    let status = validated_response.status;
    let mut response_headers = validated_response.headers;
    response_headers.append(
        "OVP-Correlation-Id",
        HeaderValue::from_bytes(correlation_id.as_bytes()).unwrap(),
    );
    let body = validated_response.body;

    (
        axum::http::status::StatusCode::from_u16(status)
            .unwrap_or(axum::http::status::StatusCode::INTERNAL_SERVER_ERROR),
        response_headers,
        body,
    )
}

fn validate_response(
    response: ureq::Response,
    method: axum::http::Method,
    spec: &openapiv3::OpenAPI,
    wayfinder_path: Option<String>,
) -> ValidatedResponse {
    let failures = vec![];
    let mut properties = vec![];
    let status = response.status();
    properties.push(TestcaseProperty {
        name: "statusCode".to_string(),
        value: status.to_string(),
    });
    let mut headers = axum::http::HeaderMap::new();
    for name in &response.headers_names() {
        // This proxy server does not support Transfer-Encoding
        if name.to_lowercase() == "transfer-encoding" {
            continue;
        }
        let key = HeaderName::from_str(name).unwrap();
        let value = response.header(name).unwrap_or("");
        let value = HeaderValue::from_str(value).unwrap_or(HeaderValue::from_static(""));
        headers.insert(key, value);
    }
    let body = response.into_string().unwrap();
    let mut validated = ValidatedResponse {
        body: body.into_bytes(),
        failures,
        headers: headers.clone(),
        method: method.clone(),
        properties,
        status,
    };

    if wayfinder_path.is_none() {
        return validated;
    }

    let wayfinder_path = wayfinder_path.unwrap();
    let path = spec.paths.paths.get(&wayfinder_path).unwrap().as_item();
    if path.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Invalid HTTP method".to_string(),
            r#type: TestcaseFailureType::PathNotFound,
        });
        return validated;
    }
    let path = path.unwrap();
    let operation = match method {
        axum::http::Method::DELETE => path.delete.as_ref(),
        axum::http::Method::GET => path.get.as_ref(),
        axum::http::Method::HEAD => path.head.as_ref(),
        axum::http::Method::OPTIONS => path.options.as_ref(),
        axum::http::Method::PATCH => path.patch.as_ref(),
        axum::http::Method::POST => path.post.as_ref(),
        axum::http::Method::PUT => path.put.as_ref(),
        axum::http::Method::TRACE => path.trace.as_ref(),
        _ => None,
    };
    if operation.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Invalid HTTP method".to_string(),
            r#type: TestcaseFailureType::InvalidHTTPMethod,
        });
        return validated;
    }
    let operation = operation.unwrap();
    if let Some(operation_id) = &operation.operation_id {
        validated.properties.push(TestcaseProperty {
            name: "operationId".to_string(),
            value: operation_id.to_string(),
        });
    }
    let spec_response = operation
        .responses
        .responses
        .get(&openapiv3::StatusCode::Code(status));
    if spec_response.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Response not found for status code".to_string(),
            r#type: TestcaseFailureType::InvalidStatusCode,
        });
        return validated;
    }
    let spec_response = spec_response.unwrap();
    let response = resolve_response(spec_response, spec);
    if response.is_none() {
        validated.failures.push(TestcaseFailure {
            text:
                "Could not find response defined inline or as a #/components/responses/ reference"
                    .to_string(),
            r#type: TestcaseFailureType::MissingResponseDefinition,
        });
        return validated;
    }
    let spec_response = response.unwrap();
    let response_content_type = headers.get("Content-Type");
    if response_content_type.is_none() && !spec_response.content.is_empty() {
        validated.failures.push(TestcaseFailure {
            text: "Response did not include a Content-Type header".to_string(),
            r#type: TestcaseFailureType::MissingContentTypeHeader,
        });
        return validated;
    }
    let response_content_type = response_content_type
        .map(|v| v.to_str().unwrap())
        .unwrap_or("");
    validated.properties.push(TestcaseProperty {
        name: "responseContentType".to_string(),
        value: response_content_type.to_string(),
    });

    // No Content-Type header but response body is not empty
    if response_content_type.is_empty() && !validated.body.is_empty() {
        validated.failures.push(TestcaseFailure {
            text: "Receieved response body when empty body is expected".to_string(),
            r#type: TestcaseFailureType::MismatchNonEmptyBody,
        });
        return validated;
    }

    // Body is empty, nothing to validate
    if validated.body.is_empty() {
        return validated;
    }

    // Body is not empty but no matching Content-Type in spec
    let spec_content = spec_response.content.get(response_content_type);
    if spec_content.is_none() {
        validated.failures.push(TestcaseFailure {
            text: format!(
                "Spec does not contain matching response for Content-Type: {}",
                response_content_type
            ),
            r#type: TestcaseFailureType::MismatchedContentTypeHeader,
        });
        return validated;
    }

    let spec_content = spec_content.unwrap();
    let schema = spec_content.schema.as_ref();
    if schema.is_none() {
        if !validated.body.is_empty() {
            validated.failures.push(TestcaseFailure {
                text: "Receieved response body when empty body is expected".to_string(),
                r#type: TestcaseFailureType::MismatchNonEmptyBody,
            });
        }
        return validated;
    }
    let schema = schema.unwrap();
    let schema = resolve_schema(schema, spec);
    if schema.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Could not find schema defined inline or as a #/components/schemas/ reference"
                .to_string(),
            r#type: TestcaseFailureType::MissingSchemaDefinition,
        });
        return validated;
    }
    let spec_schema = schema.unwrap();
    if response_content_type != "application/json" {
        debug!("Skipping JSON schema validation for non-JSON response");
        return validated;
    }
    let serde_value = serde_json::from_slice::<serde_json::Value>(&validated.body);
    if serde_value.is_err() {
        validated.failures.push(TestcaseFailure {
            text: "Failed to parse response body as JSON".to_string(),
            r#type: TestcaseFailureType::FailedJSONDeserialization,
        });
        return validated;
    }
    let serde_value = serde_value.unwrap();
    let schema_validation_failures =
        validate_schema(&serde_value, spec_schema, spec, "/".to_string());
    validated.failures.extend(schema_validation_failures);

    validated
}

fn validate_schema(
    serde_value: &serde_json::Value,
    spec_schema: &openapiv3::Schema,
    spec: &openapiv3::OpenAPI,
    json_pointer: String,
) -> Vec<TestcaseFailure> {
    let mut failures = vec![];
    match serde_value {
        serde_json::Value::Null => {
            if !spec_schema.schema_data.nullable {
                failures.push(TestcaseFailure {
                    text: format!(
                        "Received null value when null is not allowed at {}",
                        json_pointer
                    ),
                    r#type: TestcaseFailureType::FailedValidationUnexpectedNull,
                });
            }
            failures
        }
        serde_json::Value::Bool(_) => {
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) =
                &spec_schema.schema_kind
            {
                return failures;
            }
            failures.push(TestcaseFailure {
                text: format!("Received unexpected boolean at {}", json_pointer),
                r#type: TestcaseFailureType::FailedValidationUnexpectedBoolean,
            });
            failures
        }
        serde_json::Value::Number(_) => {
            // TODO: This probably needs to do a more thorough check for integer vs number
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) =
                &spec_schema.schema_kind
            {
                return failures;
            }
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) =
                &spec_schema.schema_kind
            {
                return failures;
            }
            failures.push(TestcaseFailure {
                text: format!("Received unexpected number at {}", json_pointer),
                r#type: TestcaseFailureType::FailedValidationUnexpectedNumber,
            });
            failures
        }
        serde_json::Value::String(_) => {
            if let openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) =
                &spec_schema.schema_kind
            {
                return failures;
            }
            failures.push(TestcaseFailure {
                text: format!("Received unexpected string at {}", json_pointer),
                r#type: TestcaseFailureType::FailedValidationUnexpectedString,
            });
            failures
        }
        serde_json::Value::Array(serde_array) => {
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Array(spec_array)) =
                &spec_schema.schema_kind
            {
                let items_schema = spec_array.items.as_ref();
                if items_schema.is_none() {
                    failures.push(TestcaseFailure {
                        text: "Array schema does not contain items schema".to_string(),
                        r#type: TestcaseFailureType::MissingSchemaDefinition,
                    });
                    return failures;
                }
                let items_schema = items_schema.unwrap();
                let items_schema = items_schema.clone().unbox();
                let items_schema = resolve_schema(&items_schema, spec);
                if items_schema.is_none() {
                    failures.push(TestcaseFailure {
                        text: "Could not find schema defined inline or as a #/components/schemas/ reference for array items".to_string(),
                        r#type: TestcaseFailureType::MissingSchemaDefinition,
                    });
                    return failures;
                }
                let items_schema = items_schema.unwrap();
                for (index, value) in serde_array.iter().enumerate() {
                    let json_pointer = format!("{}{}/", json_pointer, index);
                    let schema_validation_failures =
                        validate_schema(value, items_schema, spec, json_pointer);
                    failures.extend(schema_validation_failures);
                }
            }
            failures
        }
        serde_json::Value::Object(serde_object) => {
            match &spec_schema.schema_kind {
                openapiv3::SchemaKind::Type(openapiv3::Type::Object(spec_object)) => {
                    for (key, value) in serde_object.iter() {
                        let json_pointer = format!("{}{}", json_pointer, key);
                        let spec_property = spec_object.properties.get(key);
                        if spec_property.is_none() {
                            failures.push(TestcaseFailure {
                                text: format!(
                                    "Unexpected property at {}, value {}",
                                    json_pointer, value
                                ),
                                r#type: TestcaseFailureType::FailedValidationUnexpectedProperty,
                            });
                            continue;
                        }
                        let spec_property = spec_property.unwrap();
                        let spec_property = spec_property.clone().unbox();
                        let spec_property = resolve_schema(&spec_property, spec);
                        if spec_property.is_none() {
                            failures.push(TestcaseFailure {
                                text: format!("Could not find schema defined inline or as a #/components/schemas/ reference for property at {}", json_pointer),
                                r#type: TestcaseFailureType::MissingSchemaDefinition,
                            });
                            continue;
                        }
                        let spec_property = spec_property.unwrap();
                        let schema_validation_failures = validate_schema(
                            value,
                            spec_property,
                            spec,
                            format!("{}/", json_pointer),
                        );
                        failures.extend(schema_validation_failures);
                    }
                }
                openapiv3::SchemaKind::AllOf { all_of } => {
                    let schema = create_schema_for_all_of(all_of, spec);
                    let schema_validation_failures =
                        validate_schema(serde_value, &schema, spec, json_pointer);
                    failures.extend(schema_validation_failures);
                }
                _ => {
                    failures.push(TestcaseFailure {
                        text: format!(
                            "Received unsupported schema kind: {:?} at {}",
                            spec_schema.schema_kind, json_pointer
                        ),
                        r#type: TestcaseFailureType::FailedValidationUnsupportedSchemaKind,
                    });
                }
            }
            failures
        }
    }
}

fn create_schema_for_all_of(
    all_of: &[openapiv3::ReferenceOr<openapiv3::Schema>],
    spec: &openapiv3::OpenAPI,
) -> openapiv3::Schema {
    let schemas = all_of
        .iter()
        .filter_map(|schema| resolve_schema(schema, spec))
        .collect::<Vec<&openapiv3::Schema>>();

    let mut property_map = serde_json::Map::new();
    for schema in schemas.iter() {
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(spec_object)) => {
                for (key, value) in spec_object.properties.iter() {
                    let json_value = serde_json::to_value(value).unwrap();
                    property_map.insert(key.clone(), serde_json::from_value(json_value).unwrap());
                }
            }

            _ => {
                // I don't know what any of the other cases mean
                error!("Encountered non-object schema in allOf: {:?}", schema);
            }
        }
    }

    let mut serde_map = serde_json::Map::new();
    serde_map.insert("type".to_string(), "object".into());
    serde_map.insert(
        "properties".to_string(),
        serde_json::Value::Object(property_map),
    );
    // TODO: gotta populate required fields as well

    serde_json::from_value(serde_json::Value::Object(serde_map)).unwrap()
}

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    info!("Shutting down...")
}

fn resolve_response<'a>(
    response: &'a openapiv3::ReferenceOr<openapiv3::Response>,
    openapi: &'a openapiv3::OpenAPI,
) -> Option<&'a openapiv3::Response> {
    match response {
        ReferenceOr::Item(item) => Some(item),
        ReferenceOr::Reference { reference } => {
            let response_name = reference.split("#/components/responses/").nth(1);
            response_name?;
            let response_name = response_name.unwrap();
            let components = openapi.components.as_ref()?;
            let found_response = components.responses.get(response_name);
            found_response?;
            let found_response = found_response.unwrap();
            found_response.as_item()
        }
    }
}

fn resolve_schema<'a>(
    schema: &'a openapiv3::ReferenceOr<openapiv3::Schema>,
    openapi: &'a openapiv3::OpenAPI,
) -> Option<&'a openapiv3::Schema> {
    match schema {
        ReferenceOr::Item(item) => Some(item),
        ReferenceOr::Reference { reference } => {
            let schema_name = reference.split("#/components/schemas/").nth(1);
            schema_name?;
            let schema_name = schema_name.unwrap();
            let components = openapi.components.as_ref()?;
            let found_schema = components.schemas.get(schema_name);
            found_schema?;
            let found_schema = found_schema.unwrap();
            found_schema.as_item()
        }
    }
}
