use assert_cmd::{assert::Assert, Command};
use predicates::prelude::*;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[derive(Clone, Copy, Debug)]
enum Provider {
    GitHub,
    GitLab,
}

const PROVIDERS: [Provider; 2] = [Provider::GitHub, Provider::GitLab];

fn release_request(provider: Provider, args: &[&str], environment: &[(&str, &str)]) -> Value {
    let (result, body) = release_response(provider, args, environment,
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
    result.success();
    body
}

fn release_response(
    provider: Provider,
    args: &[&str],
    environment: &[(&str, &str)],
    response: &str,
) -> (Assert, Value) {
    release_responses(provider, args, environment, &[response])
}

fn release_responses(
    provider: Provider,
    args: &[&str],
    environment: &[(&str, &str)],
    responses: &[&str],
) -> (Assert, Value) {
    let responses: Vec<_> = responses
        .iter()
        .map(|response| response.to_string())
        .collect();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in responses {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "No release request received");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("Failed to accept release request: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let mut content_length = None;
            loop {
                let mut line = String::new();
                assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap();
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = Some(value.trim().parse::<usize>().unwrap());
                }
            }
            let body_len = if request_line.starts_with("POST ") {
                content_length.expect("JSON request has Content-Length")
            } else {
                assert!(request_line.starts_with("GET /api/v4/projects/123/repository/compare?"));
                content_length.unwrap_or(0)
            };
            let mut body = vec![0; body_len];
            reader.read_exact(&mut body).unwrap();
            stream.write_all(response.as_bytes()).unwrap();
            requests.push((
                request_line,
                if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice::<Value>(&body).unwrap()
                },
            ));
        }
        requests.pop().unwrap()
    });

    let dir = TempDir::new().unwrap();
    // Keep dotenv and inherited CI credentials out of the child process.
    std::fs::write(dir.path().join(".env"), "").unwrap();
    let mut command = Command::cargo_bin("svci").unwrap();
    command
        .current_dir(dir.path())
        .env_clear()
        // Exercise HTTP serialization and sending against the loopback server.
        .env("ENVIRONMENT", "production")
        .env("NO_PROXY", "*")
        .timeout(Duration::from_secs(10));
    let path = match provider {
        Provider::GitHub => {
            command
                .env("GITHUB_ACTIONS", "true")
                .env("GITHUB_API_URL", &api_url)
                .env("GITHUB_REPOSITORY", "test/repo")
                .env("GITHUB_TOKEN", "test-token")
                .env("GITHUB_SHA", "test-sha");
            "/repos/test/repo/releases"
        }
        Provider::GitLab => {
            command
                .env("GITLAB_CI", "true")
                .env("CI_API_V4_URL", format!("{api_url}/api/v4"))
                .env("CI_PROJECT_ID", "123")
                .env("CI_JOB_TOKEN", "test-token")
                .env("CI_COMMIT_SHA", "test-sha");
            "/api/v4/projects/123/releases"
        }
    };
    command
        .envs(environment.iter().copied())
        .arg("release")
        .args(args);
    let result = command.assert();
    let (request_line, body) = server.join().unwrap();
    assert_eq!(request_line, format!("POST {path} HTTP/1.1\r\n"));
    (result, body)
}

#[test]
fn release_strips_one_lowercase_v_from_name_and_default_tag() {
    for provider in PROVIDERS {
        for (input, expected) in [
            ("v1.2.3", "1.2.3"),
            ("1.2.3", "1.2.3"),
            ("vv1.2.3", "v1.2.3"),
            ("V1.2.3", "V1.2.3"),
            ("preview-v1.2.3", "preview-v1.2.3"),
        ] {
            let body = release_request(provider, &["--strip-prefix-v", input], &[]);
            assert_eq!(body["name"], expected, "{provider:?}: {input}");
            assert_eq!(body["tag_name"], expected, "{provider:?}: {input}");
        }
    }
}

#[test]
fn release_strips_name_and_explicit_tag_independently() {
    for provider in PROVIDERS {
        for (name, tag, expected_name, expected_tag) in [
            ("v1.2.3", "v1.2.3", "1.2.3", "1.2.3"),
            ("Version 1.2.3", "v1.2.3", "Version 1.2.3", "1.2.3"),
            ("v1.2.3", "1.2.4", "1.2.3", "1.2.4"),
            ("vv1.2.3", "vv1.2.4", "v1.2.3", "v1.2.4"),
            ("v1.2.3", "V1.2.4", "1.2.3", "V1.2.4"),
        ] {
            let body = release_request(provider, &["-s", "--tag-name", tag, name], &[]);
            assert_eq!(body["name"], expected_name, "{provider:?}: {name}");
            assert_eq!(body["tag_name"], expected_tag, "{provider:?}: {tag}");
        }
    }
}

#[test]
fn release_reads_strip_prefix_and_tag_name_from_environment() {
    for provider in PROVIDERS {
        let body = release_request(provider, &["v1.2.3"], &[("STRIP_PREFIX_V", "true")]);
        assert_eq!(body["name"], "1.2.3");
        assert_eq!(body["tag_name"], "1.2.3");

        let environment = [("STRIP_PREFIX_V", "true"), ("TAG_NAME", "v1.2.4")];
        let body = release_request(provider, &["v1.2.3"], &environment);
        assert_eq!(body["name"], "1.2.3");
        assert_eq!(body["tag_name"], "1.2.4");

        let body = release_request(provider, &["--tag-name", "v1.2.5", "v1.2.3"], &environment);
        assert_eq!(body["name"], "1.2.3");
        assert_eq!(body["tag_name"], "1.2.5");
    }
}

#[test]
fn release_preserves_prefixes_when_stripping_is_disabled() {
    for provider in PROVIDERS {
        for environment in [&[][..], &[("STRIP_PREFIX_V", "false")][..]] {
            let body = release_request(provider, &["v1.2.3"], environment);
            assert_eq!(body["name"], "v1.2.3");
            assert_eq!(body["tag_name"], "v1.2.3");

            let body = release_request(provider, &["--tag-name", "v1.2.4", "v1.2.3"], environment);
            assert_eq!(body["name"], "v1.2.3");
            assert_eq!(body["tag_name"], "v1.2.4");
        }
    }
}

#[test]
fn release_flag_overrides_environment_and_preserves_other_fields() {
    for provider in PROVIDERS {
        let body = release_request(
            provider,
            &[
                "--strip-prefix-v",
                "--description",
                "v-prefixed description",
                "--tag-message",
                "v-prefixed tag message",
                "v1.2.3",
            ],
            &[("STRIP_PREFIX_V", "false")],
        );
        assert_eq!(body["name"], "1.2.3");
        assert_eq!(body["tag_name"], "1.2.3");
        match provider {
            Provider::GitHub => {
                assert_eq!(body["body"], "v-prefixed description");
                assert_eq!(body["target_commitish"], "test-sha");
                assert_eq!(body["generate_release_notes"], false);
            }
            Provider::GitLab => {
                assert_eq!(body["description"], "v-prefixed description");
                assert_eq!(body["tag_message"], "v-prefixed tag message");
                assert_eq!(body["ref"], "test-sha");
            }
        }
    }
}

#[test]
fn release_sets_prerelease_for_supported_github_version_tags() {
    for provider in PROVIDERS {
        for (version, expected) in [
            ("1.2.3", false),
            ("1.2.3-rc.1", true),
            ("1.2.3-dev.2.abcd1234", true),
        ] {
            for tag in [version.to_string(), format!("v{version}")] {
                let body = release_request(provider, &[&tag], &[]);
                assert_eq!(body["tag_name"], tag);
                match provider {
                    Provider::GitHub => {
                        assert_eq!(body["prerelease"], expected, "{tag}");
                        assert_eq!(body["generate_release_notes"], false);
                    }
                    Provider::GitLab => assert!(body.get("prerelease").is_none()),
                }
            }
        }
    }
}

#[test]
fn github_release_classifies_tag_instead_of_display_name() {
    for (name, tag, expected) in [
        ("September release", "v1.2.3-rc.1", true),
        ("v1.2.3", "1.2.3-dev.2.abcd1234", true),
        ("v1.2.3-rc.1", "v1.2.3", false),
    ] {
        let body = release_request(Provider::GitHub, &["-g", "--tag-name", tag, name], &[]);
        assert_eq!(body["name"], name);
        assert_eq!(body["tag_name"], tag);
        assert_eq!(body["prerelease"], expected, "{name}: {tag}");
        assert_eq!(body["generate_release_notes"], true);
    }
}

#[test]
fn github_release_classifies_tag_after_environment_and_prefix_resolution() {
    let environment = [("TAG_NAME", "v1.2.3-rc.1"), ("STRIP_PREFIX_V", "true")];
    let body = release_request(Provider::GitHub, &["Version 1.2.3"], &environment);
    assert_eq!(body["tag_name"], "1.2.3-rc.1");
    assert_eq!(body["prerelease"], true);

    let body = release_request(
        Provider::GitHub,
        &["--tag-name", "v1.2.3", "Version 1.2.3"],
        &environment,
    );
    assert_eq!(body["tag_name"], "1.2.3");
    assert_eq!(body["prerelease"], false);

    let body = release_request(Provider::GitHub, &["--strip-prefix-v", "vv1.2.3-rc.1"], &[]);
    assert_eq!(body["tag_name"], "v1.2.3-rc.1");
    assert_eq!(body["prerelease"], true);
}

#[test]
fn github_release_preserves_other_tags_as_full_releases() {
    for tag in [
        "nightly-build",
        "preview-v1.2.3-rc.1",
        "v1.2.3-alpha.1",
        "V1.2.3-rc.1",
        "vv1.2.3-rc.1",
        "v1.2.3-rc",
        "v1.2.3-rc.x",
        "v1.2.3-rc.01",
        "v1.2.3-rc.1.extra",
        "v1.2.3-rc.1-extra",
        "v1.2.3-dev.2",
        "v1.2.3-dev.2.",
        "v1.2.3-dev.2.abcd1234.extra",
    ] {
        let body = release_request(Provider::GitHub, &[tag], &[]);
        assert_eq!(body["name"], tag);
        assert_eq!(body["tag_name"], tag);
        assert_eq!(body["prerelease"], false, "{tag}");
    }
}

#[test]
fn http_failures_report_context_without_panicking_or_success_output() {
    for provider in PROVIDERS {
        for (response, expected, cause) in [
            (
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 6\r\nConnection: close\r\n\r\ndenied",
                "Status: 403 Forbidden",
                Some("Body:\ndenied"),
            ),
            (
                "HTTP/1.1 201 Created\r\nContent-Length: 8\r\nConnection: close\r\n\r\nnot-json",
                "Failed to parse HTTP response as a JSON object",
                Some("expected ident"),
            ),
            (
                "HTTP/1.1 201 Created\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]",
                "Failed to parse HTTP response as a JSON object",
                Some("invalid type"),
            ),
            (
                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 100\r\nConnection: close\r\n\r\nshort",
                "Failed to read HTTP response body (Status: 500 Internal Server Error)",
                Some("Caused by:"),
            ),
        ] {
            let (result, _) = release_response(provider, &["v1.2.3"], &[], response);
            let result = result.code(1).stdout("")
                .stderr(predicate::str::contains("Error: "))
                .stderr(predicate::str::contains(expected))
                .stderr(predicate::str::contains("panicked at").not());
            if let Some(cause) = cause {
                result.stderr(predicate::str::contains(cause));
            }
        }
    }
}

#[test]
fn gitlab_compare_failure_preserves_release_creation_and_reports_its_cause() {
    let (result, body) = release_responses(
        Provider::GitLab,
        &[
            "release-name",
            "--generate-release-notes",
            "--previous-tag",
            "v1.2.2",
        ],
        &[("CI_PROJECT_URL", "https://example.com/test/repo")],
        &[
            "HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nnot-json",
            "HTTP/1.1 201 Created\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        ],
    );
    result
        .success()
        .stderr(predicate::str::contains(
            "Failed to parse HTTP response as a JSON object",
        ))
        .stderr(predicate::str::contains("Caused by: expected ident"))
        .stderr(predicate::str::contains("panicked at").not());
    assert_eq!(
        body["description"],
        "# What's Changed\nhttps://example.com/test/repo/-/compare/v1.2.2...test-sha"
    );
}
