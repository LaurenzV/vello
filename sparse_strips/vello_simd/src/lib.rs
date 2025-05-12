#![allow(non_camel_case_types)]
#![allow(missing_docs)]

pub mod neon;
pub mod scalar;
mod util;

use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

pub trait Simd<const C: usize> {}

pub trait Base:
    Sized + Copy + Add<Self, Output = Self> + Mul<Self, Output = Self> + Sub<Self, Output = Self> + Debug
{
}

// Unfortunately we cannot make C an associated constant instead, because generic const expressions
// are unstable and therefore we can't use them in `load` and `store`.
// We also need to explicitly define `A` for the same reason.
pub trait Narrowed<const C: usize, const A: usize>: Base {
    type Scalar: Scalar;
    type Widened: Widened<C, A, Self>;

    fn load(src: &[Self::Scalar; C]) -> Self;
    fn load_alphas(src: &[u8; A]) -> Self;
    fn load_4(src: &[Self::Scalar; 4]) -> Self;
    fn splat(value: Self::Scalar) -> Self;
    fn from_normalized_u8(value: u8) -> Self;
    fn store(self, dest: &mut [Self::Scalar; C]);
    fn widen(self) -> Self::Widened;

    #[inline(always)]
    fn zero() -> Self {
        Self::splat(Scalar::ZERO)
    }

    #[inline(always)]
    fn mid() -> Self {
        Self::splat(Scalar::MID)
    }

    #[inline(always)]
    fn one() -> Self {
        Self::splat(Scalar::ONE)
    }

    #[inline(always)]
    fn one_minus(self) -> Self {
        Self::one() - self
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        (self.widen() * other.widen()).normalize().narrow()
    }
    
    #[inline(always)]
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        self.normalized_mul(other1) + other2
    }
    
    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        let p1 = self.widen() * other1.widen();
        let p2 = other2.widen() * other3.widen();
        
        (p1 + p2).normalize().narrow()
    }
    
    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        other2 - self.normalized_mul(other1)
    }
}

pub trait Scalar: Base {
    const ZERO: Self;
    const MID: Self;
    const ONE: Self;

    fn to_rgba8(src: &[Self]) -> [u8; 4];
}

pub trait Widened<const C: usize, const A: usize, N: Narrowed<C, A>>: Base {
    fn narrow(self) -> N;
    fn normalize(self) -> Self;
}
