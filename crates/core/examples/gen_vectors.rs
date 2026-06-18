use osm_core::vectors::{TestVector, VectorArgon2};
use osm_core::{encode_standard, encode_words, encrypt, Argon2Params, FixedRng};

struct Case {
    name: &'static str,
    passphrase: &'static str,
    plaintext: &'static str,
    salt: [u8; 16],
    nonce: [u8; 12],
    params: Argon2Params,
}

fn main() {
    let cases = [
        Case {
            name: "ascii-basic",
            passphrase: "紙袋、みかん、夜道、ラジオ",
            plaintext: "secret note",
            salt: [0x11; 16],
            nonce: [0x22; 12],
            params: Argon2Params { m_cost: 8 * 1024, t_cost: 1, p_cost: 1 },
        },
        Case {
            name: "japanese-memo",
            passphrase: "あいことば",
            plaintext: "東京駅 18:30 集合 予約番号 AB-1234",
            salt: [0xAB; 16],
            nonce: [0xCD; 12],
            params: Argon2Params { m_cost: 8 * 1024, t_cost: 2, p_cost: 1 },
        },
        Case {
            name: "empty-plaintext",
            passphrase: "x",
            plaintext: "",
            salt: [0x01; 16],
            nonce: [0x02; 12],
            params: Argon2Params { m_cost: 8 * 1024, t_cost: 1, p_cost: 1 },
        },
    ];

    let vectors: Vec<TestVector> = cases
        .iter()
        .map(|c| {
            let mut seed = c.salt.to_vec();
            seed.extend_from_slice(&c.nonce);
            let mut rng = FixedRng::new(seed);
            let payload = encrypt(c.plaintext.as_bytes(), c.passphrase, c.params, &mut rng);
            TestVector {
                name: c.name.to_string(),
                passphrase: c.passphrase.to_string(),
                plaintext_utf8: c.plaintext.to_string(),
                argon2: VectorArgon2 {
                    m_cost: c.params.m_cost,
                    t_cost: c.params.t_cost,
                    p_cost: c.params.p_cost,
                },
                salt_hex: hex::encode(c.salt),
                nonce_hex: hex::encode(c.nonce),
                payload_hex: hex::encode(payload.to_bytes()),
                standard: encode_standard(&payload),
                words: encode_words(&payload),
            }
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&vectors).unwrap());
}
