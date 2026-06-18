#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(payload) = osm_core::detect_and_decode(s) {
            // Fixed passphrase; property is "no panic", result discarded.
            let _ = osm_core::decrypt(&payload, "fuzz-passphrase");
        }
    }
});
