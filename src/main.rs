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
use miette::{miette, Diagnostic, LabeledSpan, NamedSource, SourceSpan};
use openapiv3::ReferenceOr;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use thiserror::Error;
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

        /// Port to run the proxy server on
        #[arg(short, long, default_value = "3000")]
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

#[derive(Debug, Template)]
#[template(path = "junit.xml")]
struct JunitTemplate<'a> {
    testcases: &'a [Testcase],
    failed_testcases: usize,
}

#[derive(Debug)]
struct Testcase {
    name: String,
    failures: Vec<TestcaseFailure>,
    properties: Vec<TestcaseProperty>,
    time: String,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
struct TestcaseProperty {
    name: String,
    value: String,
}

#[derive(Debug)]
struct TestcaseFailure {
    text: String,
    r#type: TestcaseFailureType,
    report: Option<miette::Report>,
}

/// An enum describing the type of test failure that occurred.
#[derive(Debug, Clone)]
enum TestcaseFailureType {
    /// The HTTP method used in the request is not one of the expected values: DELETE, GET, HEAD, OPTIONS, PATCH, POST, PUT, or TRACE.
    InvalidHTTPMethod,
    /// The status code returned by the upstream server does not have a matching response in the OpenAPI spec.
    InvalidStatusCode,
    /// The OpenAPI spec contained a missing inline response definition or referenced a response that did not exist.
    MissingResponseDefinition,
    /// The requested path was not found in the OpenAPI spec. This response was not validated and may be missing relevant testcase properties.
    /// The OpenAPI spec contained a missing inline schema definition or referenced a schema that did not exist.
    MissingSchemaDefinition,
    /// The requested path was not found in the OpenAPI spec.
    PathNotFound,

    /// The request body could not be deserialized as JSON.
    RequestFailedJSONDeserialization,
    /// The request body contained a boolean value when the OpenAPI spec expected a different type.
    RequestFailedValidationUnexpectedBoolean,
    /// The request body contains a null value when the OpenAPI spec did not allow null values.
    RequestFailedValidationUnexpectedNull,
    /// The request body contained a number value when the OpenAPI spec expected a different type.
    RequestFailedValidationUnexpectedNumber,
    /// The request body contained a property that was not defined in the OpenAPI spec.
    RequestFailedValidationUnexpectedProperty,
    /// The request body contained a string value when the OpenAPI spec expected a different type.
    RequestFailedValidationUnexpectedString,
    /// The OpenAPI spec contained a schema with an unsupported kind, such as anyOf, oneOf, or not.
    RequestFailedValidationUnsupportedSchemaKind,
    /// The client included a non-empty body when the OpenAPI spec expected an empty body.
    RequestMismatchNonEmptyBody,
    /// The client included a Content-Type header in the request that does not match any content types defined in the OpenAPI spec.
    RequestMismatchedContentTypeHeader,
    /// The client did not include a Content-Type header in the request. This is only an issue when the response body is not empty.
    RequestMissingContentTypeHeader,

    /// The response body could not be deserialized as JSON.
    ResponseFailedJSONDeserialization,
    /// The response body contained a boolean value when the OpenAPI spec expected a different type.
    ResponseFailedValidationUnexpectedBoolean,
    /// The response body contains a null value when the OpenAPI spec did not allow null values.
    ResponseFailedValidationUnexpectedNull,
    /// The response body contained a number value when the OpenAPI spec expected a different type.
    ResponseFailedValidationUnexpectedNumber,
    /// The response body contained a property that was not defined in the OpenAPI spec.
    ResponseFailedValidationUnexpectedProperty,
    /// The response body contained a string value when the OpenAPI spec expected a different type.
    ResponseFailedValidationUnexpectedString,
    /// The OpenAPI spec contained a schema with an unsupported kind, such as anyOf, oneOf, or not.
    ResponseFailedValidationUnsupportedSchemaKind,
    /// The upstream server included a non-empty response body when the OpenAPI spec expected an empty body.
    ResponseMismatchNonEmptyBody,
    /// The upstream server included a Content-Type header in the response that does not match any content types defined in the OpenAPI spec.
    ResponseMismatchedContentTypeHeader,
    /// The upstream server did not include a Content-Type header in the response. This is only an issue when the response body is not empty.
    ResponseMissingContentTypeHeader,
}

impl std::fmt::Display for TestcaseFailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TestcaseFailureType::InvalidHTTPMethod => write!(f, "InvalidHTTPMethod"),
            TestcaseFailureType::InvalidStatusCode => write!(f, "InvalidStatusCode"),
            TestcaseFailureType::MissingResponseDefinition => {
                write!(f, "MissingResponseDefinition")
            }
            TestcaseFailureType::MissingSchemaDefinition => write!(f, "MissingSchemaDefinition"),
            TestcaseFailureType::PathNotFound => write!(f, "PathNotFound"),
            TestcaseFailureType::RequestFailedJSONDeserialization => {
                write!(f, "Request.FailedJSONDeserialization")
            }
            TestcaseFailureType::RequestFailedValidationUnexpectedBoolean => {
                write!(f, "Request.FailedValidation.UnexpectedBoolean")
            }
            TestcaseFailureType::RequestFailedValidationUnexpectedNull => {
                write!(f, "Request.FailedValidation.UnexpectedNull")
            }
            TestcaseFailureType::RequestFailedValidationUnexpectedNumber => {
                write!(f, "Request.FailedValidation.UnexpectedNumber")
            }
            TestcaseFailureType::RequestFailedValidationUnexpectedProperty => {
                write!(f, "Request.FailedValidation.UnexpectedProperty")
            }
            TestcaseFailureType::RequestFailedValidationUnexpectedString => {
                write!(f, "Request.FailedValidation.UnexpectedString")
            }
            TestcaseFailureType::RequestFailedValidationUnsupportedSchemaKind => {
                write!(f, "Request.FailedValidation.UnsupportedSchemaKind")
            }
            TestcaseFailureType::RequestMismatchNonEmptyBody => {
                write!(f, "Request.MismatchNonEmptyBody")
            }
            TestcaseFailureType::RequestMismatchedContentTypeHeader => {
                write!(f, "Request.MismatchedContentTypeHeader")
            }
            TestcaseFailureType::RequestMissingContentTypeHeader => {
                write!(f, "Request.MissingContentTypeHeader")
            }
            TestcaseFailureType::ResponseFailedJSONDeserialization => {
                write!(f, "Response.FailedJSONDeserialization")
            }
            TestcaseFailureType::ResponseFailedValidationUnexpectedBoolean => {
                write!(f, "Response.FailedValidation.UnexpectedBoolean")
            }
            TestcaseFailureType::ResponseFailedValidationUnexpectedNull => {
                write!(f, "Response.FailedValidation.UnexpectedNull")
            }
            TestcaseFailureType::ResponseFailedValidationUnexpectedNumber => {
                write!(f, "Response.FailedValidation.UnexpectedNumber")
            }
            TestcaseFailureType::ResponseFailedValidationUnexpectedProperty => {
                write!(f, "Response.FailedValidation.UnexpectedProperty")
            }
            TestcaseFailureType::ResponseFailedValidationUnexpectedString => {
                write!(f, "Response.FailedValidation.UnexpectedString")
            }
            TestcaseFailureType::ResponseFailedValidationUnsupportedSchemaKind => {
                write!(f, "Response.FailedValidation.UnsupportedSchemaKind")
            }
            TestcaseFailureType::ResponseMismatchNonEmptyBody => {
                write!(f, "Response.MismatchNonEmptyBody")
            }
            TestcaseFailureType::ResponseMismatchedContentTypeHeader => {
                write!(f, "Response.MismatchedContentTypeHeader")
            }
            TestcaseFailureType::ResponseMissingContentTypeHeader => {
                write!(f, "Response.MissingContentTypeHeader")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ValidationPerspective {
    Request,
    Response,
}

struct ValidatedRequest {
    body: Vec<u8>,
    path_and_query: String,
    #[allow(dead_code)]
    path: String,
    failures: Vec<TestcaseFailure>,
    headers: axum::http::HeaderMap,
    method: axum::http::Method,
    properties: Vec<TestcaseProperty>,
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
    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
             miette::MietteHandlerOpts::new()
            .terminal_links(false)
            .unicode(true)
            .context_lines(3)
            .tab_width(4)
            .break_words(false)
            .without_syntax_highlighting()
            .color(false)
            .build(),
            )
    }));



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
    let lock = state.testcases.lock().await;
    let testcases: &[Testcase] = lock.as_ref();
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

fn extract_path_remainder(upstream_path: &str, request_path: &str) -> String {
    // This function removes the upstream path from the incoming request path. We do this for 2
    // reasons:
    // 1. We need to match the request path against the OpenAPI spec, which does not include the
    // upstream path.
    // 2. It gives us a single place where we can determine if the upstream path was configured
    //    with an ending slash or not.
    let path = request_path.strip_prefix(upstream_path);
    match path {
        Some(p) => {
            // Make sure the path starts with a slash
            if p.starts_with("/") {
                p.to_string()
            } else {
                format!("/{}", p)
            }
        }
        None => request_path.to_string(),
    }
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
    let upstream_path = upstream.path();
    let path_remainder = extract_path_remainder(upstream_path, request.uri().path());

    let wayfinder_path = wayfind::Path::new(&path_remainder).unwrap();
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
                report: None,
                
            });
        }
    }
    let wayfinder_path = wayfinder_match.map(|m| m.route.to_string());
    let mut validated_request = validate_request(request, &spec, wayfinder_path.clone()).await;
    properties.append(&mut validated_request.properties);
    failures.append(&mut validated_request.failures);
    let outgoing_url = upstream.join(&path_remainder).unwrap();

    let mut outgoing_request =
        ureq::request(validated_request.method.as_str(), outgoing_url.as_str());
    for (key, value) in validated_request.headers.iter() {
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

    properties.push(TestcaseProperty {
        name: "correlationId".to_string(),
        value: correlation_id.to_string(),
    });
    let testcase_name = format!(
        "{} {} {}",
        validated_request.method.as_str(),
        validated_request.path_and_query,
        correlation_id
    );
    let body = validated_request.body;
    let time_start = std::time::Instant::now();
    let response = outgoing_request.send_bytes(&body).or_any_status().unwrap();
    let time_end = std::time::Instant::now();
    let duration = time_end - time_start;
    let mut validated_response =
        validate_response(response, validated_request.method, &spec, wayfinder_path);
    failures.append(&mut validated_response.failures);
    properties.append(&mut validated_response.properties);
    properties.sort();
    let mut cases = testcases.lock().await;
    cases.push(Testcase {
        name: testcase_name,
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

async fn validate_request(
    request: axum::http::Request<axum::body::Body>,
    spec: &openapiv3::OpenAPI,
    wayfinder_path: Option<String>,
) -> ValidatedRequest {
    let path_and_query = request.uri().path_and_query().unwrap().to_string();
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    let headers = request.headers().clone();
    let body = axum::body::to_bytes(request.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    info!(
        method = method.as_str(),
        path = path.to_string(),
        "Handling request"
    );

    let properties = vec![
        TestcaseProperty {
            name: "path".to_string(),
            value: path.to_string(),
        },
        TestcaseProperty {
            name: "method".to_string(),
            value: method.to_string(),
        },
    ];

    let mut validated = ValidatedRequest {
        body,
        path_and_query,
        path,
        failures: vec![],
        headers: headers.clone(),
        method: method.clone(),
        properties,
    };

    if wayfinder_path.is_none() {
        return validated;
    }

    let wayfinder_path = wayfinder_path.unwrap();
    let path = spec.paths.paths.get(&wayfinder_path).unwrap().as_item();
    if path.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Path not found in spec".to_string(),
            r#type: TestcaseFailureType::PathNotFound,
            report: None,
            
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
            report: None,
            
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
    let spec_request_body = operation.request_body.as_ref();
    if spec_request_body.is_none() && !validated.body.is_empty() {
        validated.failures.push(TestcaseFailure {
            text: "Client supplied request body when none was included in spec.".to_string(),
            r#type: TestcaseFailureType::RequestMismatchNonEmptyBody,
            report: None,
            
        });
        return validated;
    }
    if spec_request_body.is_none() {
        return validated;
    }
    let spec_request_body = spec_request_body.unwrap();
    let spec_request_body = resolve_request_body(spec_request_body, spec);
    if spec_request_body.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Could not find request defined inline or as a #/components/requestBodies/ reference".to_string(),
            r#type: TestcaseFailureType::MissingSchemaDefinition,
                report: None,
                
        });
        return validated;
    }
    let spec_request_body = spec_request_body.unwrap();
    let request_content_type = headers.get("Content-Type");

    // No Content-Type header but request body is not empty
    if request_content_type.is_none() && !validated.body.is_empty() {
        validated.failures.push(TestcaseFailure {
            text: "Request did not include a Content-Type header, unable to validate request body schema.".to_string(),
            r#type: TestcaseFailureType::RequestMissingContentTypeHeader,
                report: None,
                
        });
        return validated;
    }

    // No Content-Type header but request body is empty. This is fine.
    if request_content_type.is_none() && validated.body.is_empty() {
        return validated;
    }

    let request_content_type = request_content_type
        .map(|v| v.to_str().unwrap())
        .unwrap_or("");
    validated.properties.push(TestcaseProperty {
        name: "requestContentType".to_string(),
        value: request_content_type.to_string(),
    });

    let spec_content = spec_request_body.content.get(request_content_type);
    if spec_content.is_none() {
        validated.failures.push(TestcaseFailure {
            text: format!(
                "Spec does not contain matching request for Content-Type: {}",
                request_content_type
            ),
            r#type: TestcaseFailureType::RequestMismatchedContentTypeHeader,
            report: None,
            
        });
        return validated;
    }
    let spec_content = spec_content.unwrap();
    if request_content_type != "application/json" {
        debug!("Request content type is not application/json, skipping request body validation");
        return validated;
    }
    let spec_schema = spec_content.schema.as_ref();
    if spec_schema.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Could not find schema defined inline or as a #/components/schemas/ reference"
                .to_string(),
            r#type: TestcaseFailureType::MissingSchemaDefinition,
            report: None,
            
        });
        return validated;
    }
    let spec_schema = spec_schema.unwrap();
    let spec_schema = resolve_schema(spec_schema, spec);
    if spec_schema.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Could not find schema defined inline or as a #/components/schemas/ reference"
                .to_string(),
            r#type: TestcaseFailureType::MissingSchemaDefinition,
            report: None,
            
        });
        return validated;
    }
    let spec_schema = spec_schema.unwrap();
    let serde_value = serde_json::from_slice::<serde_json::Value>(&validated.body);
    if serde_value.is_err() {
        validated.failures.push(TestcaseFailure {
            text: "Failed to parse request body as JSON".to_string(),
            r#type: TestcaseFailureType::RequestFailedJSONDeserialization,
            report: None,
            
        });
        return validated;
    }
    let serde_value = serde_value.unwrap();
    let schema_validation_failures = validate_schema(
        &serde_value,
        spec_schema,
        spec,
        "/".to_string(),
        ValidationPerspective::Request,
    );
    validated.failures.extend(schema_validation_failures);

    validated
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
    let body_bytes = match status {
        204 | 304 => vec![],
        _ => {
            let mut buffer: Vec<u8> = vec![];
            // Failing to read the response body probably means a body wasn't included in the response.
            // If that's the case, just return the empty buffer.
            response.into_reader().read_to_end(&mut buffer).unwrap_or(0);
            buffer
        }
    };

    let mut validated = ValidatedResponse {
        body: body_bytes,
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
        return validated;
    }
    let operation = operation.unwrap();
    let spec_response = operation
        .responses
        .responses
        .get(&openapiv3::StatusCode::Code(status));
    if spec_response.is_none() {
        validated.failures.push(TestcaseFailure {
            text: "Response not found for status code".to_string(),
            r#type: TestcaseFailureType::InvalidStatusCode,
            report: None,
            
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
            report: None,
            
        });
        return validated;
    }
    let spec_response = response.unwrap();
    let response_content_type = headers.get("Content-Type");
    if response_content_type.is_none() && !spec_response.content.is_empty() {
        validated.failures.push(TestcaseFailure {
            text: "Response did not include a Content-Type header".to_string(),
            r#type: TestcaseFailureType::ResponseMissingContentTypeHeader,
            report: None,
            
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
            r#type: TestcaseFailureType::ResponseMismatchNonEmptyBody,
            report: None,
            
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
            r#type: TestcaseFailureType::ResponseMismatchedContentTypeHeader,
            report: None,
            
        });
        return validated;
    }

    let spec_content = spec_content.unwrap();
    let schema = spec_content.schema.as_ref();
    if schema.is_none() {
        if !validated.body.is_empty() {
            validated.failures.push(TestcaseFailure {
                text: "Receieved response body when empty body is expected".to_string(),
                r#type: TestcaseFailureType::ResponseMismatchNonEmptyBody,
                report: None,
                
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
            report: None,
            
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
            r#type: TestcaseFailureType::ResponseFailedJSONDeserialization,
            report: None,
            
        });
        return validated;
    }
    let serde_value = serde_value.unwrap();
    let schema_validation_failures = validate_schema(
        &serde_value,
        spec_schema,
        spec,
        "/".to_string(),
        ValidationPerspective::Response,
    );
    validated.failures.extend(schema_validation_failures);

    validated
}

fn validate_schema(
    serde_value: &serde_json::Value,
    spec_schema: &openapiv3::Schema,
    spec: &openapiv3::OpenAPI,
    json_pointer: String,
    validation_perspective: ValidationPerspective,
) -> Vec<TestcaseFailure> {
    let mut failures = vec![];
    match serde_value {
        serde_json::Value::Null => {
            if !spec_schema.schema_data.nullable {
                let failure_type = match validation_perspective {
                    ValidationPerspective::Request => {
                        TestcaseFailureType::RequestFailedValidationUnexpectedNull
                    }
                    ValidationPerspective::Response => {
                        TestcaseFailureType::ResponseFailedValidationUnexpectedNull
                    }
                };
                failures.push(TestcaseFailure {
                    text: format!(
                        "Received null value when null is not allowed at {}",
                        json_pointer
                    ),
                    r#type: failure_type,
                    report: None,
                    
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
            let serde_string = serde_value.to_string();

            let m = miette!(
                labels = vec![miette::LabeledSpan::at_offset(0, "here")],
                "messed up bool"
            );
            m.with_source_code(serde_string);
            let failure_type = match validation_perspective {
                ValidationPerspective::Request => {
                    TestcaseFailureType::RequestFailedValidationUnexpectedBoolean
                }
                ValidationPerspective::Response => {
                    TestcaseFailureType::ResponseFailedValidationUnexpectedBoolean
                }
            };
            failures.push(TestcaseFailure {
                text: format!("Received unexpected boolean at {}", json_pointer),
                r#type: failure_type,
                report: None,
                
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
            let failure_type = match validation_perspective {
                ValidationPerspective::Request => {
                    TestcaseFailureType::RequestFailedValidationUnexpectedNumber
                }
                ValidationPerspective::Response => {
                    TestcaseFailureType::ResponseFailedValidationUnexpectedNumber
                }
            };
            failures.push(TestcaseFailure {
                text: format!("Received unexpected number at {}", json_pointer),
                r#type: failure_type,
                report: None,
                
            });
            failures
        }
        serde_json::Value::String(_) => {
            if let openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) =
                &spec_schema.schema_kind
            {
                return failures;
            }
            let failure_type = match validation_perspective {
                ValidationPerspective::Request => {
                    TestcaseFailureType::RequestFailedValidationUnexpectedString
                }
                ValidationPerspective::Response => {
                    TestcaseFailureType::ResponseFailedValidationUnexpectedString
                }
            };
            failures.push(TestcaseFailure {
                text: format!("Received unexpected string at {}", json_pointer),
                r#type: failure_type,
                report: None,
                
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
                        report: None,
                        
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
                report: None,
                
                    });
                    return failures;
                }
                let items_schema = items_schema.unwrap();
                for (index, value) in serde_array.iter().enumerate() {
                    let json_pointer = format!("{}{}/", json_pointer, index);
                    let schema_validation_failures = validate_schema(
                        value,
                        items_schema,
                        spec,
                        json_pointer,
                        validation_perspective,
                    );
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
                            let failure_type = match validation_perspective {
                                ValidationPerspective::Request => {
                                    TestcaseFailureType::RequestFailedValidationUnexpectedProperty
                                }
                                ValidationPerspective::Response => {
                                    TestcaseFailureType::ResponseFailedValidationUnexpectedProperty
                                }
                            };
                            let report = miette!(
                                labels = vec![miette::LabeledSpan::at_offset(0, "here")],
                                "messed up property"
                                ).with_source_code(serde_value.to_string());
                            failures.push(TestcaseFailure {
                                text: format!(
                                    "Unexpected property at {}, value {}",
                                    json_pointer, value
                                ),
                                r#type: failure_type,
                                report: Some(report),
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
                report: None,
                
                            });
                            continue;
                        }
                        let spec_property = spec_property.unwrap();
                        let schema_validation_failures = validate_schema(
                            value,
                            spec_property,
                            spec,
                            format!("{}/", json_pointer),
                            validation_perspective,
                        );
                        failures.extend(schema_validation_failures);
                    }
                }
                openapiv3::SchemaKind::AllOf { all_of } => {
                    let schema = create_schema_for_all_of(all_of, spec);
                    let schema_validation_failures = validate_schema(
                        serde_value,
                        &schema,
                        spec,
                        json_pointer,
                        validation_perspective,
                    );
                    failures.extend(schema_validation_failures);
                }
                _ => {
                    let failure_type = match validation_perspective {
                        ValidationPerspective::Request => {
                            TestcaseFailureType::RequestFailedValidationUnsupportedSchemaKind
                        }
                        ValidationPerspective::Response => {
                            TestcaseFailureType::ResponseFailedValidationUnsupportedSchemaKind
                        }
                    };
                    failures.push(TestcaseFailure {
                        text: format!(
                            "Received unsupported schema kind: {:?} at {}",
                            spec_schema.schema_kind, json_pointer
                        ),
                        r#type: failure_type,
                        report: None,
                        
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

fn resolve_request_body<'a>(
    request_body: &'a openapiv3::ReferenceOr<openapiv3::RequestBody>,
    openapi: &'a openapiv3::OpenAPI,
) -> Option<&'a openapiv3::RequestBody> {
    match request_body {
        ReferenceOr::Item(item) => Some(item),
        ReferenceOr::Reference { reference } => {
            let request_body_name = reference.split("#/components/requestBodies/").nth(1);
            request_body_name?;
            let request_body_name = request_body_name.unwrap();
            let components = openapi.components.as_ref()?;
            let found_request_body = components.request_bodies.get(request_body_name);
            found_request_body?;
            let found_request_body = found_request_body.unwrap();
            found_request_body.as_item()
        }
    }
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
