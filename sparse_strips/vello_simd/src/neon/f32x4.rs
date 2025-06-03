use crate::neon::u32x4::u32x4;
use crate::{Base, ColorLike, Float, Type, Widened};
use std::arch::aarch64::*;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub(crate) struct f32x4(pub(crate) float32x4_t);

impl Add for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_f32(self.0, rhs.0)) }
    }
}

impl Mul for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { Self(vmulq_f32(self.0, rhs.0)) }
    }
}

impl Sub for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self(vsubq_f32(self.0, rhs.0)) }
    }
}

impl Div for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: Self) -> Self::Output {
        unsafe { Self(vdivq_f32(self.0, rhs.0)) }
    }
}

impl Base for f32x4 {}

impl Type for f32x4 {
    type Scalar = f32;
    type Widened = Self;
    type Float = Self;
    const IS_FLOAT: bool = false;
    const LENGTH: usize = 0;

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self::from_normalized_u8(src[0])
    }

    #[inline(always)]
    fn splat_4(src: [f32; 4]) -> Self {
        unsafe { Self(vld1q_f32(src.as_ptr())) }
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe { Self(vdupq_n_f32(value)) }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline(always)]
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmaq_f32(other2.0, self.0, other1.0)) }
    }

    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        self.normalized_mul_add(other1, other2 * other3)
    }

    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmsq_f32(other2.0, self.0, other1.0)) }
    }

    #[inline(always)]
    fn min(self, other: Self) -> Self {
        unsafe { Self(vminq_f32(self.0, other.0)) }
    }

    #[inline(always)]
    fn max(self, other: Self) -> Self {
        unsafe { Self(vmaxq_f32(self.0, other.0)) }
    }

    #[inline(always)]
    fn splat_4th_element(self) -> Self {
        unsafe {
            let z0 = vzip2q_f32(self.0, self.0);
            let z1 = vzip2q_f32(z0, z0);

            Self(z1)
        }
    }

    #[inline(always)]
    fn load(src: &[f32]) -> Self {
        let src: &[f32; 4] = src.try_into().unwrap();

        unsafe {
            let loaded = vld1q_f32(src.as_ptr());

            Self(loaded)
        }
    }

    #[inline(always)]
    fn load_alphas_f32(src: &[f32]) -> Self {
        unsafe { Self(vdupq_n_f32(src[0])) }
    }

    #[inline(always)]
    fn load_f32_many(src: &[f32]) -> Self {
        todo!()
    }

    #[inline(always)]
    fn splat_color<T: ColorLike>(color: T) -> Self {
        Self::load(&color.to_rgbf32())
    }

    #[inline(always)]
    fn splat_alpha<T: ColorLike>(color: T) -> Self {
        Self::splat(color.to_rgbf32()[3])
    }

    #[inline(always)]
    fn store(self, dest: &mut [Self::Scalar]) {
        let dest: &mut [f32; 4] = dest.try_into().unwrap();

        unsafe {
            vst1q_f32(dest.as_mut_ptr(), self.0);
        }
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        self
    }

    #[inline(always)]
    fn from_float(f: &[Self::Float]) -> Self {
        f[0]
    }
}

impl Widened<f32x4> for f32x4 {
    #[inline(always)]
    fn narrow(self) -> f32x4 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }

    #[inline(always)]
    fn clamp(self) -> Self {
        unsafe {
            let min = vdupq_n_f32(0.0);
            let max = vdupq_n_f32(1.0);

            Self(vmaxq_f32(vminq_f32(self.0, max), min))
        }
    }
}

impl Float for f32x4 {
    type Integer = u32x4;

    #[inline(always)]
    fn sqrt(self) -> Self {
        Self(unsafe { vsqrtq_f32(self.0) })
    }

    #[inline(always)]
    fn powf(mut self, exponent: f32) -> Self {
        // TODO: SIMDify
        let mut storage = [0.0; 4];
        unsafe {
            vst1q_f32(storage.as_mut_ptr(), self.0);

            storage[0] = storage[0].powf(exponent);
            storage[1] = storage[1].powf(exponent);
            storage[2] = storage[2].powf(exponent);
            storage[3] = storage[3].powf(exponent);

            let loaded = vld1q_f32(storage.as_ptr());

            Self(loaded)
        }
    }

    #[inline(always)]
    fn abs(self) -> Self {
        unsafe { Self(vabsq_f32(self.0)) }
    }

    #[inline(always)]
    fn floor(self) -> Self {
        unsafe { Self(vrndmq_f32(self.0)) }
    }
    
    #[inline(always)]
    fn trunc(self) -> Self {
        unsafe { Self(vrndq_f32(self.0)) }
    }
    
    #[inline(always)]
    fn reinterpret(self) -> Self::Integer {
        unsafe { u32x4(vreinterpretq_u32_f32(self.0)) }
    }

    #[inline(always)]
    fn fract(mut self) -> Self {
        unsafe {
            let c1 = vcvtq_s32_f32(self.0);
            let c2 = vcvtq_f32_s32(c1);

            Self(vsubq_f32(self.0, c2))
        }
    }

    #[inline(always)]
    fn lt(self, other: Self) -> u32x4 {
        u32x4(unsafe { vcltq_f32(self.0, other.0) })
    }

    #[inline(always)]
    fn leq(self, other: Self) -> u32x4 {
        u32x4(unsafe { vcleq_f32(self.0, other.0) })
    }

    #[inline(always)]
    fn ne(self, other: Self) -> u32x4 {
        u32x4(unsafe { vmvnq_u32(vceqq_f32(self.0, other.0)) })
    }

    #[inline(always)]
    fn if_then_else(mask: u32x4, a: Self, b: Self) -> Self {
        unsafe { Self(vbslq_f32(mask.0, a.0, b.0)) }
    }

    #[inline(always)]
    fn splat_col_pos(pos: (f32, f32), _: (f32, f32), y_advance: (f32, f32)) -> (Self, Self) {
        let (x_pos, y_pos) = splat_col_pos(pos, y_advance);

        (Self(x_pos), Self(y_pos))
    }
}

impl f32x4 {
    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self::from_normalized_u8(src[0])
    }

    #[inline(always)]
    fn splat_4(src: &[f32; 4]) -> Self {
        unsafe { Self(vld1q_f32(src.as_ptr())) }
    }

    #[inline(always)]
    pub(crate) fn splat(value: f32) -> Self {
        unsafe { Self(vdupq_n_f32(value)) }
    }

    #[inline(always)]
    pub(crate) fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline(always)]
    pub(crate) fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmaq_f32(other2.0, self.0, other1.0)) }
    }

    #[inline(always)]
    pub(crate) fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        self.normalized_mul_add(other1, other2 * other3)
    }

    #[inline(always)]
    pub(crate) fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmsq_f32(other2.0, self.0, other1.0)) }
    }

    #[inline(always)]
    pub(crate) fn min(self, other: Self) -> Self {
        unsafe { Self(vminq_f32(self.0, other.0)) }
    }

    #[inline(always)]
    pub(crate) fn max(self, other: Self) -> Self {
        unsafe { Self(vmaxq_f32(self.0, other.0)) }
    }

    #[inline(always)]
    pub(crate) fn abs(self) -> Self {
        unsafe { Self(vabsq_f32(self.0)) }
    }

    #[inline(always)]
    pub(crate) fn floor(self) -> Self {
        unsafe { Self(vrndmq_f32(self.0)) }
    }

    #[inline(always)]
    pub(crate) fn fract(self) -> Self {
        unsafe {
            let c1 = vcvtq_s32_f32(self.0);
            let c2 = vcvtq_f32_s32(c1);

            Self(vsubq_f32(self.0, c2))
        }
    }

    #[inline(always)]
    pub(crate) fn splat_4th_element(self) -> Self {
        unsafe {
            let z0 = vzip2q_f32(self.0, self.0);
            let z1 = vzip2q_f32(z0, z0);

            Self(z1)
        }
    }

    #[inline(always)]
    pub(crate) fn lt(self, other: Self) -> u32x4 {
        u32x4(unsafe { vcltq_f32(self.0, other.0) })
    }

    #[inline(always)]
    pub(crate) fn leq(self, other: Self) -> u32x4 {
        u32x4(unsafe { vcleq_f32(self.0, other.0) })
    }

    #[inline(always)]
    pub(crate) fn ne(self, other: Self) -> u32x4 {
        u32x4(unsafe { vmvnq_u32(vceqq_f32(self.0, other.0)) })
    }

    #[inline(always)]
    pub(crate) fn if_then_else(mask: u32x4, a: Self, b: Self) -> Self {
        unsafe { Self(vbslq_f32(mask.0, a.0, b.0)) }
    }
}

#[inline(always)]
fn splat_col_pos(pos: (f32, f32), advance: (f32, f32)) -> (float32x4_t, float32x4_t) {
    unsafe {
        let column_mask = vld1q_f32([0.0, 1.0, 2.0, 3.0].as_ptr());

        let x_positions = vfmaq_f32(vdupq_n_f32(pos.0), column_mask, vdupq_n_f32(advance.0));
        let y_positions = vfmaq_f32(vdupq_n_f32(pos.1), column_mask, vdupq_n_f32(advance.1));

        (x_positions, y_positions)
    }
}
