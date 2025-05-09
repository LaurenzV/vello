use crate::{Numerical, Float};
use std::arch::aarch64::*;
use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub struct f32x4(float32x4_t);

impl Add for f32x4 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vaddq_f32(self.0, rhs.0))
        }
    }
}

impl Mul for f32x4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vmulq_f32(self.0, rhs.0))
        }
    }
}

impl Sub for f32x4 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vsubq_f32(self.0, rhs.0))
        }
    }
}

impl Float<4> for f32x4 {
    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe {
            Self(vdupq_n_f32(value))
        }
    }

    #[inline(always)]
    fn load(src: &[f32; 4]) -> Self {
        unsafe {
            Self(vld1q_f32(src.as_ptr()))
        }
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32; 4]) {
        unsafe {
            vst1q_f32(dest.as_mut_ptr(), self.0)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct f32x8(float32x4x2_t);

impl Add for f32x8 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_f32(self.0.0, rhs.0.0);
            self.0.1 = vaddq_f32(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Mul for f32x8 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_f32(self.0.0, rhs.0.0);
            self.0.1 = vmulq_f32(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Sub for f32x8 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_f32(self.0.0, rhs.0.0);
            self.0.1 = vsubq_f32(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Numerical for f32x8 {}

impl Float<8> for f32x8 {
    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe {
            let v = vdupq_n_f32(value);
            Self(float32x4x2_t(v, v))
        }
    }

    #[inline(always)]
    fn load(src: &[f32; 8]) -> Self {
        unsafe {
            Self(vld1q_f32_x2(src.as_ptr()))
        }
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32; 8]) {
        unsafe {
            vst1q_f32_x2(dest.as_mut_ptr(), self.0)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct f32x16(float32x4x4_t);

impl Add for f32x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_f32(self.0.0, rhs.0.0);
            self.0.1 = vaddq_f32(self.0.1, rhs.0.1);
            self.0.2 = vaddq_f32(self.0.2, rhs.0.2);
            self.0.3 = vaddq_f32(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Mul for f32x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_f32(self.0.0, rhs.0.0);
            self.0.1 = vmulq_f32(self.0.1, rhs.0.1);
            self.0.2 = vmulq_f32(self.0.2, rhs.0.2);
            self.0.3 = vmulq_f32(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Sub for f32x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_f32(self.0.0, rhs.0.0);
            self.0.1 = vsubq_f32(self.0.1, rhs.0.1);
            self.0.2 = vsubq_f32(self.0.2, rhs.0.2);
            self.0.3 = vsubq_f32(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Numerical for f32x16 {}

impl Float<16> for f32x16 {
    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe {
            let v = vdupq_n_f32(value);
            Self(float32x4x4_t(v, v, v, v))
        }
    }

    #[inline(always)]
    fn load(src: &[f32; 16]) -> Self {
        unsafe {
            Self(vld1q_f32_x4(src.as_ptr()))
        }
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32; 16]) {
        unsafe {
            vst1q_f32_x4(dest.as_mut_ptr(), self.0)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u16x8(uint16x8_t);

impl Add for u16x8 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vaddq_u16(self.0, rhs.0))
        }
    }
}

impl Mul for u16x8 {
    type Output = u16x8;

    fn mul(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vmulq_u16(self.0, rhs.0))
        }
    }
}

impl Sub for u16x8 {
    type Output = u16x8;

    fn sub(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vsubq_u16(self.0, rhs.0))
        }
    }
}

impl Numerical for u16x8 {}

#[derive(Copy, Clone, Debug)]
pub struct u16x16(uint16x8x2_t);

impl Add for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_u16(self.0.0, rhs.0.0);
            self.0.1 = vaddq_u16(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Mul for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_u16(self.0.0, rhs.0.0);
            self.0.1 = vmulq_u16(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Sub for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_u16(self.0.0, rhs.0.0);
            self.0.1 = vsubq_u16(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Numerical for u16x16 {}

#[derive(Copy, Clone, Debug)]
pub struct u16x32(uint16x8x4_t);

impl Add for u16x32 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_u16(self.0.0, rhs.0.0);
            self.0.1 = vaddq_u16(self.0.1, rhs.0.1);
            self.0.2 = vaddq_u16(self.0.2, rhs.0.2);
            self.0.3 = vaddq_u16(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Mul for u16x32 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_u16(self.0.0, rhs.0.0);
            self.0.1 = vmulq_u16(self.0.1, rhs.0.1);
            self.0.2 = vmulq_u16(self.0.2, rhs.0.2);
            self.0.3 = vmulq_u16(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Sub for u16x32 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_u16(self.0.0, rhs.0.0);
            self.0.1 = vsubq_u16(self.0.1, rhs.0.1);
            self.0.2 = vsubq_u16(self.0.2, rhs.0.2);
            self.0.3 = vsubq_u16(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Numerical for u16x32 {}

#[derive(Copy, Clone, Debug)]
pub struct u8x16(uint8x16_t);

impl Add for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vaddq_u8(self.0, rhs.0))
        }
    }
}

impl Mul for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vmulq_u8(self.0, rhs.0))
        }
    }
}

impl Sub for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe {
            Self(vsubq_u8(self.0, rhs.0))
        }
    }
}

impl Numerical for u8x16 {}

#[derive(Copy, Clone, Debug)]
pub struct u8x32(uint8x16x2_t);

impl Add for u8x32 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_u8(self.0.0, rhs.0.0);
            self.0.1 = vaddq_u8(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Mul for u8x32 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_u8(self.0.0, rhs.0.0);
            self.0.1 = vmulq_u8(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Sub for u8x32 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_u8(self.0.0, rhs.0.0);
            self.0.1 = vsubq_u8(self.0.1, rhs.0.1);
            
            self
        }
    }
}

impl Numerical for u8x32 {}

#[derive(Copy, Clone, Debug)]
pub struct u8x64(uint8x16x4_t);

impl Add for u8x64 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_u8(self.0.0, rhs.0.0);
            self.0.1 = vaddq_u8(self.0.1, rhs.0.1);
            self.0.2 = vaddq_u8(self.0.2, rhs.0.2);
            self.0.3 = vaddq_u8(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Mul for u8x64 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_u8(self.0.0, rhs.0.0);
            self.0.1 = vmulq_u8(self.0.1, rhs.0.1);
            self.0.2 = vmulq_u8(self.0.2, rhs.0.2);
            self.0.3 = vmulq_u8(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Sub for u8x64 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_u8(self.0.0, rhs.0.0);
            self.0.1 = vsubq_u8(self.0.1, rhs.0.1);
            self.0.2 = vsubq_u8(self.0.2, rhs.0.2);
            self.0.3 = vsubq_u8(self.0.3, rhs.0.3);
            
            self
        }
    }
}

impl Numerical for u8x64 {}