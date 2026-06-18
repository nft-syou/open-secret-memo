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
    let outcome = decrypt("not a ciphertext", "pw");
    assert!(!outcome.ok);
    assert_eq!(outcome.error_kind, "malformed");
}
