use wasm_bindgen_test::*;
use osm_wasm::{decrypt, encrypt};

#[wasm_bindgen_test]
fn roundtrip_standard() {
    // m_cost kept low for a fast test.
    let ct = encrypt("hello", "pw", 8192, 1, 1, "standard").unwrap();
    assert!(ct.starts_with("OSM1."));
    let outcome = decrypt(&ct, "pw");
    assert!(outcome.ok);
    assert_eq!(outcome.text, "hello");
}

#[wasm_bindgen_test]
fn wrong_passphrase_reports_auth_failed() {
    let ct = encrypt("hello", "right", 8192, 1, 1, "standard").unwrap();
    let outcome = decrypt(&ct, "wrong");
    assert!(!outcome.ok);
    assert_eq!(outcome.error_kind, "auth_failed");
}

#[wasm_bindgen_test]
fn malformed_input_reports_malformed() {
    // "OSM1." prefix routes to the standard decoder; "!!!" is not valid base64url.
    let outcome = decrypt("OSM1.!!!", "pw");
    assert!(!outcome.ok);
    assert_eq!(outcome.error_kind, "malformed");
}

#[wasm_bindgen_test]
fn unknown_word_reports_invalid_word() {
    // No "OSM" prefix => routes to the Japanese-wordlist decoder; this token is not in the list.
    let outcome = decrypt("zzzznotaword", "pw");
    assert!(!outcome.ok);
    assert_eq!(outcome.error_kind, "invalid_word");
    assert_eq!(outcome.error_word, "zzzznotaword");
}
