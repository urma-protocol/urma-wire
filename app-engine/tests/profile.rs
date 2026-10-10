use bitcoin::hashes::{sha256, Hash};
use serde_json::Value;
use std::{fs, path::PathBuf};
use urma_core::format::{RecordKind, Urma};
use wire_app_core::publication::Content;
use wire_app_engine::record::{build, review};

#[test]
fn profile_boundaries_and_multibyte_spellings_preserve_exact_octets() {
    assert_eq!(Urma::MAX_PROFILE_BYTES, 128);
    for text in [
        String::new(),
        "x".repeat(127),
        "x".repeat(128),
        "é".repeat(64),
        "😀".repeat(32),
        format!("{}xx", "e\u{301}".repeat(42)),
        "  昵称\0e\u{301}\r\n".into(),
    ] {
        let content = Content::Nickname(text.clone());
        let record = build(&content).unwrap().encode().unwrap();
        assert_eq!(&record[..8], &RecordKind::Profile.prefix());
        assert_eq!(&record[8..], text.as_bytes());
        assert_eq!(record.len(), 8 + text.len());
        assert!(record.len() <= 136);
        assert_eq!(review(&record).unwrap(), content);
    }
    for text in [
        "x".repeat(129),
        "界".repeat(43),
        "😀".repeat(33),
        "x".repeat(32760),
    ] {
        let original = text.clone();
        let error = build(&Content::Nickname(text.clone())).unwrap_err();
        assert!(error
            .to_string()
            .contains("profile body exceeds 128 UTF-8 bytes"));
        assert_eq!(text, original);
    }
}

fn fixture(value: &Value) -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../urma/tests/vectors");
    let bytes = fs::read(root.join(value["file"].as_str().unwrap())).unwrap();
    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        value["bytes"].as_u64().unwrap()
    );
    assert_eq!(
        sha256::Hash::hash(&bytes).to_string(),
        value["sha256"].as_str().unwrap()
    );
    bytes
}

#[test]
fn independent_utf8_reference_is_consumed_by_the_shared_native_browser_codec() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../urma/tests/vectors");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let reference: Value =
        serde_json::from_slice(&fixture(&manifest["profile_reference"])).unwrap();
    assert_eq!(reference["maximum_body_bytes"], 128);
    assert_eq!(reference["maximum_record_bytes"], 136);
    assert_eq!(reference["normalization"], "none");
    assert_eq!(reference["truncation"], "none");
    for case in reference["cases"].as_array().unwrap() {
        let record = fixture(&case["record"]);
        let body = fixture(&case["body"]);
        assert_eq!(&record[8..], body, "{}", case["name"]);
        if case["outcome"] == "valid" {
            let content = Content::Nickname(case["text"].as_str().unwrap().into());
            assert_eq!(review(&record).unwrap(), content, "{}", case["name"]);
            assert_eq!(build(&content).unwrap().encode().unwrap(), record);
        } else {
            assert!(review(&record).is_err(), "{}", case["name"]);
            if let Some(text) = case["text"].as_str() {
                assert!(
                    build(&Content::Nickname(text.into())).is_err(),
                    "{}",
                    case["name"]
                );
            }
        }
    }
}
