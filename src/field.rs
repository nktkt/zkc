use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

pub const MODULUS: u64 = 18_446_744_073_709_551_557;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldElement(u64);

impl FieldElement {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn from_i128(value: i128) -> Self {
        let modulus = MODULUS as i128;
        let reduced = value.rem_euclid(modulus);
        Self(reduced as u64)
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        let parsed = raw
            .parse::<i128>()
            .map_err(|err| format!("invalid field element `{raw}`: {err}"))?;
        Ok(Self::from_i128(parsed))
    }
}

impl fmt::Display for FieldElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for FieldElement {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let value = (self.0 as u128 + rhs.0 as u128) % (MODULUS as u128);
        Self(value as u64)
    }
}

impl Sub for FieldElement {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.0 >= rhs.0 {
            Self(self.0 - rhs.0)
        } else {
            Self(MODULUS - (rhs.0 - self.0))
        }
    }
}

impl Mul for FieldElement {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let value = (self.0 as u128 * rhs.0 as u128) % (MODULUS as u128);
        Self(value as u64)
    }
}

impl Neg for FieldElement {
    type Output = Self;

    fn neg(self) -> Self::Output {
        if self.0 == 0 {
            self
        } else {
            Self(MODULUS - self.0)
        }
    }
}
