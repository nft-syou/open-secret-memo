use osm_core::{
    decode_standard, decode_words, decrypt, encode_standard, encode_words, encrypt, Argon2Params,
    FixedRng,
};
use proptest::prelude::*;

fn fast() -> Argon2Params {
    Argon2Params { m_cost: 8 * 1024, t_cost: 1, p_cost: 1 }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn encrypt_decrypt_roundtrip(plaintext: Vec<u8>, passphrase in ".{1,64}") {
        let mut rng = FixedRng::new(vec![1, 2, 3, 4, 5, 6, 7]);
        let payload = encrypt(&plaintext, &passphrase, fast(), &mut rng);
        prop_assert_eq!(decrypt(&payload, &passphrase).unwrap(), plaintext);
    }

    #[test]
    fn standard_encode_decode_roundtrip(plaintext: Vec<u8>) {
        let mut rng = FixedRng::new(vec![9, 8, 7, 6, 5]);
        let payload = encrypt(&plaintext, "pw", fast(), &mut rng);
        let s = encode_standard(&payload);
        prop_assert_eq!(decode_standard(&s).unwrap(), payload);
    }

    #[test]
    fn words_encode_decode_roundtrip(plaintext: Vec<u8>) {
        let mut rng = FixedRng::new(vec![3, 1, 4, 1, 5, 9, 2]);
        let payload = encrypt(&plaintext, "pw", fast(), &mut rng);
        let w = encode_words(&payload);
        prop_assert_eq!(decode_words(&w).unwrap(), payload);
    }
}
