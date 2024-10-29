use httpmock::MockServer;
use insta_cmd::get_cargo_bin;
use rand::Rng;
use std::process::Command;
use ureq::OrAnyStatus;

/// This struct is used to start the validator proxy.
struct ValidatorProxyServerHandle {
    process: std::process::Child,
}

impl ValidatorProxyServerHandle {
    /// new will start the validator proxy on a random part using the petstore.yaml file.
    fn new(url: &str, port: u16) -> Self {
        let mut cmd = Command::new(get_cargo_bin("openapi-validator-proxy"));
        cmd.args([
            "proxy",
            "tests/petstore.yaml",
            url,
            "--port",
            &port.to_string(),
        ]);
        let child = cmd.spawn().unwrap();
        // Wait for the server to start
        std::thread::sleep(std::time::Duration::from_millis(1000));
        println!("Proxy server started");
        Self { process: child }
    }
}

impl Drop for ValidatorProxyServerHandle {
    /// This Drop implementation will kill the validator proxy server when the handle goes out of scope (when the test ends).
    fn drop(&mut self) {
        self.process.kill().unwrap();
    }
}

#[test]
fn path_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/pet");
        then.status(404).body("Not Found");
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pet", port).as_str())
        .set("OVP-Correlation-Id", "path_not_found")
        .call()
        .or_any_status()?;
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn invalid_http_method() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::DELETE).path("/pets");
        then.status(405).body("Method Not Allowed");
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::delete(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "invalid_http_method")
        .call()
        .or_any_status()?;
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn invalid_status_code() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/pets");
        then.status(600).body("Server Error");
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "invalid_status_code")
        .call()
        .or_any_status()?;
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn missing_content_type_header() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/pets");
        then.status(200).body(r#"[{"id": 1, "name": "dog"}]"#);
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "missing_content_type_header")
        .call()
        .or_any_status()?;
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn mismatched_content_type_header() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/pets");
        then.status(200)
            .header("Content-Type", "wrong")
            .body(r#"[{"id": 1, "name": "dog"}]"#);
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "mismatched_content_type_header")
        .call()
        .or_any_status()?;
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn mismatch_non_empty_body() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(202)
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "mismatch_non_empty_body")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn missing_schema_definition() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/missing_pets_schema");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!([]));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/missing_pets_schema", port).as_str())
        .set("OVP-Correlation-Id", "missing_schema_definition")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn failed_json_deserialization() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with the value of the `id` field missing
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(200)
            .header("Content-Type", "application/json")
            .body(r#"{"id":, "name": "dog"}"#);
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "failed_json_deserialization")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn failed_validation_unexpected_null() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with the value of the `id` field missing
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": null, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_null")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn failed_validation_unexpected_boolean() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with boolean `id` instead of integer
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": false, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_boolean")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn failed_validation_unexpected_number() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with number `name` instead of string
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": 0}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_number")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn failed_validation_unexpected_string() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with string `id` instead of integer
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": "1", "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_string")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn failed_validation_unexpected_property() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with extra field
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog", "extra": "field"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set(
            "OVP-Correlation-Id",
            "failed_validation_unexpected_property",
        )
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn failed_validation_unsupported_schema_kind() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with extra field
        when.method(httpmock::Method::GET)
            .path("/any_of_pet_schema");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/any_of_pet_schema", port).as_str())
        .set(
            "OVP-Correlation-Id",
            "failed_validation_unsupported_schema_kind",
        )
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn delete_with_204() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with extra field
        when.method(httpmock::Method::DELETE).path("/pets/1");
        then.status(204);
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::delete(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "delete_with_204")
        .call()
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    insta::assert_snapshot!(xml);
    Ok(())
}
