/// Abstraction over randomness so tests and test-vector generation are deterministic.
pub trait Rng {
    fn fill(&mut self, buf: &mut [u8]);
}

/// Production RNG backed by the OS CSPRNG.
pub struct OsRng;

impl Rng for OsRng {
    fn fill(&mut self, buf: &mut [u8]) {
        use rand_core::{OsRng as Backing, RngCore};
        Backing.fill_bytes(buf);
    }
}

/// Deterministic RNG that emits a fixed byte sequence, cycling if exhausted.
/// ONLY for tests and test-vector generation — never for real encryption.
pub struct FixedRng {
    bytes: Vec<u8>,
    pos: usize,
}

impl FixedRng {
    pub fn new(bytes: Vec<u8>) -> Self {
        assert!(!bytes.is_empty(), "FixedRng needs at least one byte");
        FixedRng { bytes, pos: 0 }
    }
}

impl Rng for FixedRng {
    fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.bytes[self.pos % self.bytes.len()];
            self.pos += 1;
        }
    }
}
