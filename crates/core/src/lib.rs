mod crypto;
mod error;
mod params;
mod payload;
mod rng;

pub use crypto::{decrypt, encrypt, normalize_passphrase};
pub use error::{DecryptError, FormatError};
pub use params::{Argon2Params, ParamError, M_COST_MAX, M_COST_MIN, P_COST_MIN, T_COST_MIN};
pub use payload::{Payload, HEADER_LEN, MAGIC};
pub use rng::{FixedRng, OsRng, Rng};
