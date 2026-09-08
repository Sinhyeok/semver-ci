// Load the production modules directly because svci is a binary-only crate.
#[allow(dead_code)]
#[path = "../src"]
mod source {
    pub(crate) mod errors;
}
#[path = "../src/models/semantic_version.rs"]
mod semantic_version;

use semantic_version::SemanticVersion;
use source::errors;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

fn parse(version: &str) -> SemanticVersion {
    SemanticVersion::from_string(version.to_string()).unwrap()
}

#[test]
fn supported_versions_round_trip_with_and_without_prefix() {
    for version in ["0.0.0", "12.34.56", "1.2.3-rc.10", "1.2.3-dev.7.abcd1234"] {
        let prefixed = format!("v{version}");
        for input in [version, prefixed.as_str()] {
            let parsed = parse(input);
            assert_eq!(parsed.to_string(false), version, "input: {input}");
            assert_eq!(parsed.to_string(true), prefixed, "input: {input}");
        }
    }
}

#[test]
fn invalid_versions_report_the_invalid_component() {
    for (input, expected) in [
        ("", "Invalid version string format"),
        ("1.2", "Invalid version string format"),
        ("1.2.3.4", "Invalid version string format"),
        ("x.2.3", "Invalid major version"),
        ("1.x.3", "Invalid minor version"),
        ("1.2.x", "Invalid patch version"),
        ("18446744073709551616.2.3", "Invalid major version"),
        ("1.18446744073709551616.3", "Invalid minor version"),
        ("1.2.18446744073709551616", "Invalid patch version"),
        ("1.2.3-", "Invalid metadata format"),
        ("1.2.3-rc", "Invalid metadata format"),
        ("1.2.3-rc.x", "Invalid prerelease number"),
        ("1.2.3-rc.", "Invalid prerelease number"),
        ("1.2.3-rc.18446744073709551616", "Invalid prerelease number"),
    ] {
        let error = SemanticVersion::from_string(input.to_string()).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "input: {input}, error: {error}"
        );
    }
}

#[test]
fn version_components_accept_u64_max() {
    let version = parse(
        "18446744073709551615.18446744073709551615.18446744073709551615-rc.18446744073709551615",
    );
    assert_eq!(version.major, u64::MAX);
    assert_eq!(version.minor, u64::MAX);
    assert_eq!(version.patch, u64::MAX);
    assert_eq!(version.prerelease_number, u64::MAX);
}

#[test]
fn ordering_uses_numeric_components_then_prerelease_precedence() {
    for (lower, higher) in [
        ("2.99.99", "10.0.0"),
        ("1.2.99", "1.10.0"),
        ("1.2.9", "1.2.10"),
        ("1.2.3-dev.99.abcdef12", "1.2.3-rc.1"),
        ("1.2.3-rc.2", "1.2.3-rc.10"),
        ("1.2.3-dev.2.ffffffff", "1.2.3-dev.10.00000000"),
        ("1.2.3-rc.99", "1.2.3"),
        ("1.2.3", "1.2.4-dev.1.abcdef12"),
    ] {
        let (lower, higher) = (parse(lower), parse(higher));
        assert_eq!(lower.cmp(&higher), Ordering::Less);
        assert_eq!(higher.cmp(&lower), Ordering::Greater);
        assert_eq!(lower.partial_cmp(&higher), Some(Ordering::Less));
        assert_eq!(lower.cmp_precedence(&higher), Ordering::Less);
        assert_eq!(higher.cmp_precedence(&lower), Ordering::Greater);
    }
    let version = parse("1.2.3");
    assert_eq!(version.cmp(&version), Ordering::Equal);
    assert_eq!(version.cmp_precedence(&version), Ordering::Equal);
}

#[test]
fn different_dev_shas_have_equal_precedence_but_distinct_identity_and_order() {
    let a = parse("1.2.3-dev.1.aaaaaaaa");
    let b = parse("1.2.3-dev.1.bbbbbbbb");
    assert_ne!(a, b);
    assert_eq!(a.cmp(&b), Ordering::Less);
    assert_eq!(b.cmp(&a), Ordering::Greater);
    assert_eq!(a.cmp_precedence(&b), Ordering::Equal);
    assert_eq!(b.cmp_precedence(&a), Ordering::Equal);
}

#[test]
fn ordering_obeys_comparison_laws_for_supported_versions() {
    let mut versions: Vec<_> = [
        "0.0.0",
        "1.2.3-dev.1.aaaaaaaa",
        "v1.2.3-dev.1.aaaaaaaa",
        "1.2.3-dev.1.bbbbbbbb",
        "1.2.3-dev.2.00000000",
        "1.2.3-dev.10.ffffffff",
        "1.2.3-rc.1",
        "v1.2.3-rc.1",
        "1.2.3-rc.2",
        "1.2.3-rc.10",
        "1.2.3",
        "v1.2.3",
        "1.2.4-dev.1.aaaaaaaa",
        "1.3.0",
        "2.0.0",
        "18446744073709551615.0.0",
    ]
    .into_iter()
    .map(parse)
    .collect();
    // Prerelease calculation also stores a SHA on RC values before formatting.
    let mut rc = parse("1.2.3-rc.1");
    rc.commit_short_sha = "aaaaaaaa".to_string();
    versions.push(rc);

    for a in &versions {
        for b in &versions {
            let ordering = a.cmp(b);
            assert_eq!(ordering == Ordering::Equal, a == b, "{a:?}, {b:?}");
            assert_eq!(ordering, b.cmp(a).reverse(), "{a:?}, {b:?}");
            assert_eq!(a.partial_cmp(b), Some(ordering), "{a:?}, {b:?}");
            for c in &versions {
                if a <= b && b <= c {
                    assert!(a <= c, "{a:?}, {b:?}, {c:?}");
                }
            }
        }
    }
}

#[test]
fn ordered_collections_preserve_distinct_versions_and_deduplicate_identical_ones() {
    let names = [
        "1.2.3-dev.1.aaaaaaaa",
        "1.2.3-dev.1.bbbbbbbb",
        "1.2.3-rc.1",
        "1.2.3",
    ];
    let mut set = BTreeSet::new();
    let mut map = BTreeMap::new();
    for (index, name) in names.iter().enumerate() {
        assert!(set.insert(parse(name)), "{name}");
        assert_eq!(map.insert(parse(name), index), None, "{name}");
    }
    assert_eq!(set.len(), names.len());
    assert_eq!(map.len(), names.len());
    assert!(!set.contains(&parse("1.2.3-dev.1.cccccccc")));
    assert_eq!(map.get(&parse("1.2.3-dev.1.cccccccc")), None);

    for (index, name) in names.iter().enumerate() {
        let identical = parse(&format!("v{name}"));
        assert!(set.contains(&identical));
        assert!(!set.insert(identical.clone()));
        assert_eq!(map.insert(identical.clone(), index + 10), Some(index));
        assert_eq!(map.get(&identical), Some(&(index + 10)));
    }
    let expected: Vec<_> = names.into_iter().map(parse).collect();
    assert_eq!(set.into_iter().collect::<Vec<_>>(), expected);
    assert_eq!(map.into_keys().collect::<Vec<_>>(), expected);
}

#[test]
fn scope_bumps_reset_lower_components_and_preserve_the_source() {
    for (scope, expected) in [("major", "2.0.0"), ("minor", "1.3.0"), ("patch", "1.2.4")] {
        let source = parse("1.2.3");
        assert_eq!(
            source.increase_by_scope(scope.to_string()).unwrap(),
            parse(expected)
        );
        assert_eq!(source, parse("1.2.3"));
    }
}

#[test]
fn prerelease_bump_preserves_stage_sha_and_source() {
    let source = parse("1.2.3-dev.9.abcdef12");
    assert_eq!(
        source.increase_by_scope("prerelease".to_string()).unwrap(),
        parse("1.2.3-dev.10.abcdef12")
    );
    assert_eq!(source, parse("1.2.3-dev.9.abcdef12"));
}

#[test]
fn release_clears_all_prerelease_fields_and_preserves_the_source() {
    let source = parse("1.2.3-dev.9.abcdef12");
    let released = source.release();
    assert_eq!(released, parse("1.2.3"));
    assert_eq!(released.release(), released);
    assert_eq!(source, parse("1.2.3-dev.9.abcdef12"));
}

#[test]
fn unsupported_scope_is_rejected() {
    let error = parse("1.2.3")
        .increase_by_scope("invalid".to_string())
        .unwrap_err();
    assert_eq!(error.to_string(), "Invalid scope: invalid");
}

#[test]
fn default_is_an_official_zero_version() {
    assert_eq!(SemanticVersion::default(), parse("0.0.0"));
}

#[test]
fn parse_official_and_prerelease_versions() {
    let v = SemanticVersion::from_string("1.2.3".to_string()).unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.prerelease_stage, "");
    assert_eq!(v.prerelease_number, 0);
    assert_eq!(v.commit_short_sha, "");

    let rc = SemanticVersion::from_string("1.2.3-rc.4".to_string()).unwrap();
    assert_eq!(rc.prerelease_stage, "rc");
    assert_eq!(rc.prerelease_number, 4);
    assert_eq!(rc.to_string(true), "v1.2.3-rc.4");

    let dev = SemanticVersion::from_string("1.2.3-dev.7.abcd1234".to_string()).unwrap();
    assert_eq!(dev.prerelease_stage, "dev");
    assert_eq!(dev.prerelease_number, 7);
    assert_eq!(dev.commit_short_sha, "abcd1234");
    assert_eq!(dev.to_string(false), "1.2.3-dev.7.abcd1234");
}

#[test]
fn increase_and_release_behaviors() {
    let v = SemanticVersion::from_string("1.2.3-rc.1".to_string()).unwrap();

    let minor = v.increase_by_scope("minor".to_string()).unwrap();
    assert_eq!(minor.to_string(false), "1.3.0-rc.1");

    let patch = v.increase_by_scope("patch".to_string()).unwrap();
    assert_eq!(patch.to_string(false), "1.2.4-rc.1");

    let pre = v.increase_by_scope("prerelease".to_string()).unwrap();
    assert_eq!(pre.to_string(false), "1.2.3-rc.2");

    let rel = v.release();
    assert_eq!(rel.to_string(true), "v1.2.3");
}

#[test]
fn overflow_returns_an_error_without_changing_the_source() {
    for (version, scope) in [
        ("18446744073709551615.2.3", "major"),
        ("1.18446744073709551615.3", "minor"),
        ("1.2.18446744073709551615", "patch"),
        ("1.2.3-rc.18446744073709551615", "prerelease"),
    ] {
        let source = parse(version);
        let error = source.increase_by_scope(scope.to_string()).unwrap_err();
        assert!(error.to_string().contains("exceeds u64::MAX"), "{error}");
        assert_eq!(source.to_string(false), version);
    }
}
