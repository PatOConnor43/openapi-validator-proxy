mod common;
use common::ValidatorProxyServerHandle;

use httpmock::MockServer;
use rand::Rng;
use ureq::OrAnyStatus;

#[test]
fn mismatch_non_empty_body() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with string `id` instead of integer
        when.method(httpmock::Method::GET).path("/pets/1");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::get(format!("http://localhost:{}/pets/1", port).as_str())
        .set("OVP-Correlation-Id", "mismatch_non_empty_body")
        .set("Content-Type", "application/json")
        .send_string(r#"{"intentionally": "bogus"}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn missing_schema_definition() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/missing_pets_schema");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/missing_pets_schema", port).as_str())
        .set("OVP-Correlation-Id", "missing_schema_definition")
        .set("Content-Type", "application/json")
        .send_string(r#"{}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn missing_content_type_header() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!([{"id": 1, "name": "dog"}]));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "missing_content_type_header")
        // Missing header
        //.set("Content-Type", "application/json")
        .send_string(r#"{"name": "dog"}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn mismatched_content_type_header() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!([{"id": 1, "name": "dog"}]));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "mismatched_content_type_header")
        .set("Content-Type", "text/plain")
        .send_string(r#"name: dog"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn failed_json_deserialization() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!([{"id": 1, "name": "dog"}]));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "failed_json_deserialization")
        .set("Content-Type", "application/json")
        .send_string(r#"{intentionally invalid}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn failed_validation_unexpected_number() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with string `id` instead of integer
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "0"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_number")
        .set("Content-Type", "application/json")
        .send_string(r#"{"name": 0}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn failed_validation_unexpected_null() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with the value of the `id` field missing
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_null")
        .set("Content-Type", "application/json")
        .send_string(r#"{"name": null}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn failed_validation_unexpected_boolean() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with boolean `id` instead of integer
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_boolean")
        .set("Content-Type", "application/json")
        .send_string(r#"{"name": true}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn failed_validation_unexpected_string() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with string `id` instead of integer
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set("OVP-Correlation-Id", "failed_validation_unexpected_string")
        .set("Content-Type", "application/json")
        .send_string(r#"{"id": "dog"}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn failed_validation_unexpected_property() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        // Prepare mock response with extra field
        when.method(httpmock::Method::POST).path("/pets");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/pets", port).as_str())
        .set(
            "OVP-Correlation-Id",
            "failed_validation_unexpected_property",
        )
        .set("Content-Type", "application/json")
        .send_string(r#"{"name": "dog", "extra": "field"}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}

#[test]
fn failed_validation_unsupported_schema_kind() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/any_of_pet_schema");
        then.status(201)
            .header("Content-Type", "application/json")
            .json_body(serde_json::json!({"id": 1, "name": "dog"}));
    });
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8000..u16::MAX);
    let _proxy_handle = ValidatorProxyServerHandle::new(&mock_server.url(""), port);

    ureq::post(format!("http://localhost:{}/any_of_pet_schema", port).as_str())
        .set(
            "OVP-Correlation-Id",
            "failed_validation_unsupported_schema_kind",
        )
        .set("Content-Type", "application/json")
        .send_string(r#"{"name": "dog"}"#)
        .or_any_status()
        .expect("Failed to make request");
    let junit = ureq::get(format!("http://localhost:{}/_ovp/junit", port).as_str()).call()?;
    let xml = junit.into_string()?;
    mock.assert();

    // Remove the time found at the end of the testcase xml element
    insta::with_settings!({filters => vec![
        (r#"time="0.\d{2}">"#, r#"time="0.00">"#),
    ]}, {
        insta::assert_snapshot!(xml);
    });
    Ok(())
}
