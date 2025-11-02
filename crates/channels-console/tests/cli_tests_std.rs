#[cfg(test)]
pub mod tests {
    use std::process::Command;

    #[test]
    fn test_basic_output() {
        let output = Command::new("cargo")
            .args([
                "run",
                "-p",
                "channels-console-std-test",
                "--example",
                "basic_std",
                "--features",
                "channels-console",
            ])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "Command failed with status: {}",
            output.status
        );

        assert!(!output.stderr.is_empty(), "Stderr is empty");
        let all_expected = ["examples/basic_std.rs", "bounded[10]"];

        let stdout = String::from_utf8_lossy(&output.stdout);
        for expected in all_expected {
            assert!(
                stdout.contains(expected),
                "Expected:\n{expected}\n\nGot:\n{stdout}",
            );
        }
    }

    #[test]
    fn test_basic_json_output() {
        let output = Command::new("cargo")
            .args([
                "run",
                "-p",
                "channels-console-std-test",
                "--example",
                "basic_json_std",
                "--features",
                "channels-console",
            ])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "Command failed with status: {}",
            output.status
        );

        let all_expected = ["\"label\": \"unbounded\"", "\"label\": \"bounded\""];

        let stdout = String::from_utf8_lossy(&output.stdout);

        for expected in all_expected {
            assert!(
                stdout.contains(expected),
                "Expected:\n{expected}\n\nGot:\n{stdout}",
            );
        }
    }

    #[test]
    fn test_closed_channels_output() {
        let output = Command::new("cargo")
            .args([
                "run",
                "-p",
                "channels-console-std-test",
                "--example",
                "closed_std",
                "--features",
                "channels-console",
            ])
            .output()
            .expect("Failed to execute command");

        assert!(
            output.status.success(),
            "Command failed with status: {}",
            output.status
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        let closed_count = stdout.matches("| closed").count();
        assert_eq!(
            closed_count, 2,
            "Expected 'closed' state to appear 2 times in table (bounded and unbounded), found {}.\nOutput:\n{}",
            closed_count, stdout
        );
    }

    #[test]
    fn test_metrics_endpoint() {
        use std::{process::Command, thread::sleep, time::Duration};

        // Spawn example process
        let mut child = Command::new("cargo")
            .args([
                "run",
                "-p",
                "channels-console-std-test",
                "--example",
                "basic_std",
                "--features",
                "channels-console",
            ])
            .spawn()
            .expect("Failed to spawn command");

        let mut json_text = String::new();
        let mut last_error = None;

        for _attempt in 0..4 {
            sleep(Duration::from_millis(500));

            match ureq::get("http://127.0.0.1:6770/metrics").call() {
                Ok(response) => {
                    json_text = response
                        .into_string()
                        .expect("Failed to read response body");
                    last_error = None;
                    break;
                }
                Err(e) => {
                    last_error = Some(format!("Request error: {}", e));
                }
            }
        }

        if let Some(error) = last_error {
            let _ = child.kill();
            panic!("Failed after 4 retries: {}", error);
        }

        let all_expected = ["basic_std.rs", "bounded[10]"];
        for expected in all_expected {
            assert!(
                json_text.contains(expected),
                "Expected:\n{expected}\n\nGot:\n{json_text}",
            );
        }

        let _ = child.kill();
        let _ = child.wait();
    }
}
