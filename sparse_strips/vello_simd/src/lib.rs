#![allow(non_camel_case_types)]

pub mod neon;
pub mod scalar;

use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

trait Simd<const C: usize> {}

pub trait Numerical:
    Sized
    + Copy
    + Add<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Debug
{
}

pub trait Float<const C: usize> {
    fn splat(value: f32) -> Self;
    fn store(self, dest: &mut [f32; C]);
    fn load(src: &[f32; C]) -> Self;
    fn load_4(src: &[f32; 4]) -> Self;
}

pub trait Integer<const C: usize>: Numerical {
    fn splat(value: u8) -> Self;
    fn store(self, dest: &mut [u8; C]);
    fn load(src: &[u8; C]) -> Self;
    fn load_4(src: &[u8; 4]) -> Self;
}
