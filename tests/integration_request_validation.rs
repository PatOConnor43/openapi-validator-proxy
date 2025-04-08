mod common;
use common::ValidatorProxyServerHandle;

use httpmock::MockServer;
use rand::Rng;
use ureq::OrAnyStatus;

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
