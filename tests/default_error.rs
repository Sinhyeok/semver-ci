// Load from the source directory so nested modules use their normal file paths.
#[allow(dead_code)]
#[path = "../src"]
mod source {
    pub(crate) mod errors;
}

use source::errors::{DefaultError, Result, ResultExt};
use std::error::Error;

#[test]
fn report_preserves_every_context_and_the_original_typed_cause() {
    let result: Result<u64> = "bad-number"
        .parse::<u64>()
        .context("Invalid version component")
        .context("Cannot calculate release");
    let error = result.unwrap_err();
    assert_eq!(error.to_string(), "Cannot calculate release");
    let inner = error.source().unwrap();
    assert_eq!(inner.to_string(), "Invalid version component");
    assert!(inner.source().unwrap().is::<std::num::ParseIntError>());
    assert_eq!(error.report(), "Error: Cannot calculate release\n    Caused by: Invalid version component\n    Caused by: invalid digit found in string");
}

#[test]
fn errors_without_causes_and_successful_results_remain_simple() {
    let error = DefaultError::new("Invalid release target");
    assert!(error.source().is_none());
    assert_eq!(error.report(), "Error: Invalid release target");
    assert_eq!("42".parse::<u64>().context("Invalid number").unwrap(), 42);
}

#[test]
fn errors_can_cross_thread_boundaries() {
    let error = DefaultError::new("Cannot read configuration").with_source(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "access denied",
    ));
    let returned = std::thread::spawn(move || error).join().unwrap();
    assert_eq!(
        returned
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
}
