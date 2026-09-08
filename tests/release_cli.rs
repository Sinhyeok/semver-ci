use assert_cmd::Command;
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
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
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
        let mut body = vec![0; content_length.expect("JSON request has Content-Length")];
        reader.read_exact(&mut body).unwrap();
        stream
            .write_all(
                b"HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
            )
            .unwrap();
        (
            request_line,
            serde_json::from_slice::<Value>(&body).unwrap(),
        )
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
    result.success();
    assert_eq!(request_line, format!("POST {path} HTTP/1.1\r\n"));
    body
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
