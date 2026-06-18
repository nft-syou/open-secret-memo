use std::io::{Read, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use osm_core::vectors::load_vectors;
use osm_core::{
    decrypt, detect_and_decode, encode_standard, encode_words, encrypt, Argon2Params, FixedRng,
    OsRng,
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

fn read_stdin() -> Vec<u8> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).expect("read stdin");
    buf
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Encrypt { passphrase, m_cost, t_cost, p_cost, words } => {
            let params = Argon2Params { m_cost, t_cost, p_cost };
            if let Err(e) = params.validate() {
                eprintln!("invalid parameters: {e}");
                return ExitCode::FAILURE;
            }
            let plaintext = read_stdin();
            let mut rng = OsRng;
            let payload = encrypt(&plaintext, &passphrase, params, &mut rng);
            let out = if words { encode_words(&payload) } else { encode_standard(&payload) };
            println!("{out}");
            ExitCode::SUCCESS
        }
        Command::Decrypt { passphrase } => {
            let input = String::from_utf8(read_stdin()).unwrap_or_default();
            let payload = match detect_and_decode(&input) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
            match decrypt(&payload, &passphrase) {
                Ok(plaintext) => {
                    std::io::stdout().write_all(&plaintext).unwrap();
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Verify { vectors } => {
            let json = std::fs::read_to_string(&vectors).expect("read vectors file");
            for v in load_vectors(&json) {
                let salt = hex_decode(&v.salt_hex);
                let nonce = hex_decode(&v.nonce_hex);
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

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex"))
        .collect()
}
