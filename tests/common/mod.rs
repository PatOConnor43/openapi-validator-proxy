use insta_cmd::get_cargo_bin;
use std::process::Command;

/// This struct is used to start the validator proxy.
pub struct ValidatorProxyServerHandle {
    process: std::process::Child,
}

impl ValidatorProxyServerHandle {
    /// new will start the validator proxy on a random part using the petstore.yaml file.
    pub fn new(url: &str, port: u16) -> Self {
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
