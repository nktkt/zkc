use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use num_bigint::{BigInt, BigUint, Sign};

pub const MODULUS: &str =
    "21888242871839275222246405745257275088548364400416034343698204186575808495617";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldElement(Fr);

impl FieldElement {
    pub fn zero() -> Self {
        Self(Fr::from(0u64))
    }

    pub fn one() -> Self {
        Self(Fr::from(1u64))
    }

    pub fn from_i128(value: i128) -> Self {
        if value >= 0 {
            Self::from_biguint(&BigUint::from(value as u128))
        } else {
            -Self::from_biguint(&BigUint::from(value.unsigned_abs()))
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        let parsed = BigInt::parse_bytes(raw.as_bytes(), 10)
            .ok_or_else(|| format!("invalid field element `{raw}`"))?;

        let (sign, bytes) = parsed.to_bytes_le();
        let magnitude = BigUint::from_bytes_le(&bytes);
        let value = Self::from_biguint(&magnitude);
        Ok(match sign {
            Sign::Minus => -value,
            Sign::NoSign | Sign::Plus => value,
        })
    }

    pub fn fits_in_bits(self, bits: u8) -> bool {
        if bits == 0 {
            return self == Self::zero();
        }

        let upper_bound = BigUint::from(1u8) << usize::from(bits);
        self.to_biguint() < upper_bound
    }

    pub fn to_biguint(self) -> BigUint {
        let canonical = self.0.into_bigint();
        BigUint::from_bytes_le(&canonical.to_bytes_le())
    }

    pub fn from_backend(value: Fr) -> Self {
        Self(value)
    }

    pub fn into_backend(self) -> Fr {
        self.0
    }

    fn from_biguint(value: &BigUint) -> Self {
        Self(Fr::from_le_bytes_mod_order(&value.to_bytes_le()))
    }
}

impl fmt::Display for FieldElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_biguint())
    }
}

impl Add for FieldElement {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for FieldElement {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul for FieldElement {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Neg for FieldElement {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
