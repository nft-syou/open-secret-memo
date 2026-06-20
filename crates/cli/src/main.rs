use std::io::{Read, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use osm_core::vectors::load_vectors;
use osm_core::{
    decrypt, detect_and_decode, encode_standard, encode_words, encode_words_kanji, encrypt,
    Argon2Params, FixedRng, OsRng,
};

#[derive(Parser)]
#[command(name = "osm", about = "Open Secret Memo CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Encrypt a memo read from stdin; prints the ciphertext to stdout.
    Encrypt {
        #[arg(long)]
        passphrase: String,
        #[arg(long, default_value_t = 65536)]
        m_cost: u32,
        #[arg(long, default_value_t = 1)]
        t_cost: u32,
        #[arg(long, default_value_t = 1)]
        p_cost: u8,
        /// Output the Japanese wordlist form instead of standard.
        #[arg(long)]
        words: bool,
        /// Output the experimental kanji-mixed form instead of standard.
        #[arg(long)]
        kanji: bool,
    },
    /// Decrypt a ciphertext read from stdin; prints the plaintext to stdout.
    Decrypt {
        #[arg(long)]
        passphrase: String,
    },
    /// Verify that this build reproduces a test-vector.json file.
    Verify {
        #[arg(long)]
        vectors: String,
    },
}

fn read_stdin() -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;
    Ok(buf)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Encrypt { passphrase, m_cost, t_cost, p_cost, words, kanji } => {
            let params = Argon2Params { m_cost, t_cost, p_cost };
            if let Err(e) = params.validate() {
                eprintln!("invalid parameters: {e}");
                return ExitCode::FAILURE;
            }
            let plaintext = match read_stdin() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("failed to read stdin: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let mut rng = OsRng;
            let payload = encrypt(&plaintext, &passphrase, params, &mut rng);
            let out = if kanji {
                encode_words_kanji(&payload)
            } else if words {
                encode_words(&payload)
            } else {
                encode_standard(&payload)
            };
            println!("{out}");
            ExitCode::SUCCESS
        }
        Command::Decrypt { passphrase } => {
            let raw = match read_stdin() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("failed to read stdin: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let input = match String::from_utf8(raw) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("input is not valid UTF-8 text");
                    return ExitCode::FAILURE;
                }
            };
            let payload = match detect_and_decode(&input) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
            match decrypt(&payload, &passphrase) {
                Ok(plaintext) => {
                    if let Err(e) = std::io::stdout().write_all(&plaintext) {
                        eprintln!("failed to write output: {e}");
                        return ExitCode::FAILURE;
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Verify { vectors } => {
            let json = match std::fs::read_to_string(&vectors) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("failed to read vectors file: {e}");
                    return ExitCode::FAILURE;
                }
            };
            for v in load_vectors(&json) {
                let salt = match hex_decode(&v.salt_hex) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("failed to decode hex in vector {}: {e}", v.name);
                        return ExitCode::FAILURE;
                    }
                };
                let nonce = match hex_decode(&v.nonce_hex) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("failed to decode hex in vector {}: {e}", v.name);
                        return ExitCode::FAILURE;
                    }
                };
                let mut seed = salt;
                seed.extend_from_slice(&nonce);
                let mut rng = FixedRng::new(seed);
                let payload =
                    encrypt(v.plaintext_utf8.as_bytes(), &v.passphrase, v.argon2.to_params(), &mut rng);
                if encode_standard(&payload) != v.standard {
                    eprintln!("MISMATCH in vector {}", v.name);
                    return ExitCode::FAILURE;
                }
            }
            println!("all vectors verified");
            ExitCode::SUCCESS
        }
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err(format!("odd-length hex string ({} chars)", s.len()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at offset {i}: {e}"))
        })
        .collect()
}
