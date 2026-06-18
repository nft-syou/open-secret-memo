use thiserror::Error;

/// Minimum Argon2 memory cost in KiB (8 MiB).
pub const M_COST_MIN: u32 = 8 * 1024;
/// Maximum Argon2 memory cost in KiB (1 GiB) — guards against unusable browser memory requests.
pub const M_COST_MAX: u32 = 1024 * 1024;
pub const T_COST_MIN: u32 = 1;
pub const P_COST_MIN: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2Params {
    /// Memory cost in KiB.
    pub m_cost: u32,
    /// Number of iterations.
    pub t_cost: u32,
    /// Degree of parallelism (lanes).
    pub p_cost: u8,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Argon2Params { m_cost: 65536, t_cost: 1, p_cost: 1 }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParamError {
    #[error("m_cost {0} KiB out of range [{M_COST_MIN}, {M_COST_MAX}]")]
    MCost(u32),
    #[error("t_cost {0} below minimum {T_COST_MIN}")]
    TCost(u32),
    #[error("p_cost {0} below minimum {P_COST_MIN}")]
    PCost(u8),
}

impl Argon2Params {
    pub fn validate(&self) -> Result<(), ParamError> {
        if self.m_cost < M_COST_MIN || self.m_cost > M_COST_MAX {
            return Err(ParamError::MCost(self.m_cost));
        }
        if self.t_cost < T_COST_MIN {
            return Err(ParamError::TCost(self.t_cost));
        }
        if self.p_cost < P_COST_MIN {
            return Err(ParamError::PCost(self.p_cost));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_owasp() {
        let p = Argon2Params::default();
        assert_eq!(p, Argon2Params { m_cost: 65536, t_cost: 1, p_cost: 1 });
        assert!(p.validate().is_ok());
    }

    #[test]
    fn rejects_low_memory() {
        let p = Argon2Params { m_cost: 1024, ..Default::default() };
        assert_eq!(p.validate(), Err(ParamError::MCost(1024)));
    }

    #[test]
    fn rejects_excessive_memory() {
        let p = Argon2Params { m_cost: M_COST_MAX + 1, ..Default::default() };
        assert_eq!(p.validate(), Err(ParamError::MCost(M_COST_MAX + 1)));
    }
}
