use assert_cmd::Command;

#[test]
fn encrypt_then_decrypt_roundtrip() {
    // Encrypt reads the memo from stdin, passphrase from --passphrase.
    let assert = Command::cargo_bin("osm")
        .unwrap()
        .args(["encrypt", "--passphrase", "test-pass", "--m-cost", "8192"])
        .write_stdin("my secret")
        .assert()
        .success();
    let ciphertext = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let ciphertext = ciphertext.trim().to_string();
    assert!(ciphertext.starts_with("OSM1."));

    Command::cargo_bin("osm")
        .unwrap()
        .args(["decrypt", "--passphrase", "test-pass"])
        .write_stdin(ciphertext)
        .assert()
        .success()
        .stdout("my secret");
}

#[test]
fn encrypt_kanji_then_decrypt_roundtrip() {
    let assert = Command::cargo_bin("osm")
        .unwrap()
        .args(["encrypt", "--passphrase", "k", "--m-cost", "8192", "--kanji"])
        .write_stdin("漢字メモ")
        .assert()
        .success();
    let ct = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    Command::cargo_bin("osm")
        .unwrap()
        .args(["decrypt", "--passphrase", "k"])
        .write_stdin(ct.trim().to_string())
        .assert()
        .success()
        .stdout("漢字メモ");
}

#[test]
fn decrypt_wrong_passphrase_fails() {
    let assert = Command::cargo_bin("osm")
        .unwrap()
        .args(["encrypt", "--passphrase", "right", "--m-cost", "8192"])
        .write_stdin("data")
        .assert()
        .success();
    let ciphertext = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    Command::cargo_bin("osm")
        .unwrap()
        .args(["decrypt", "--passphrase", "wrong"])
        .write_stdin(ciphertext.trim().to_string())
        .assert()
        .failure();
}

#[test]
fn verify_passes_on_bundled_vectors() {
    Command::cargo_bin("osm")
        .unwrap()
        .args(["verify", "--vectors", "../../spec/test-vector.json"])
        .assert()
        .success();
}

#[test]
fn decrypt_non_utf8_stdin_errors_cleanly() {
    Command::cargo_bin("osm")
        .unwrap()
        .args(["decrypt", "--passphrase", "pw"])
        .write_stdin(vec![0xff, 0xfe, 0x00])
        .assert()
        .failure()
        .stderr(predicates::str::contains("UTF-8"));
}
