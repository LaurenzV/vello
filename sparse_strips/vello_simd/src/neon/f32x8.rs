use crate::neon::f32x4::f32x4;
use crate::neon::u32x8::u32x8;
use crate::{Base, ColorLike, Float, Type, Widened, arith_ops};
use std::arch::aarch64::*;
use std::ops::Div;

#[derive(Copy, Clone, Debug)]
pub(crate) struct f32x8(pub(crate) f32x4, pub(crate) f32x4);

arith_ops!(f32x8);

impl Div for f32x8 {
    type Output = Self;

    #[inline(always)]
    fn div(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 / rhs.0;
        self.1 = self.1 / rhs.1;

        self
    }
}

impl Base for f32x8 {}

impl Type for f32x8 {
    type Scalar = f32;
    type Widened = Self;
    type Float = Self;

    const LENGTH: usize = 8;

    #[inline(always)]
    fn load(src: &[f32]) -> Self {
        let src: &[f32; 8] = src.try_into().unwrap();

        unsafe {
            let loaded = vld1q_f32_x2(src.as_ptr());

            Self(f32x4(loaded.0), f32x4(loaded.1))
        }
    }

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self(
            f32x4::load_alphas(&src[0..1]),
            f32x4::load_alphas(&src[1..2]),
        )
    }

    #[inline(always)]
    fn splat_4(src: [f32; 4]) -> Self {
        unsafe {
            let v = f32x4::splat_4(src);

            Self(v, v)
        }
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe {
            let v = f32x4::splat(value);

            Self(v, v)
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32]) {
        let dest: &mut [f32; 8] = dest.try_into().unwrap();

        let stored = float32x4x2_t(self.0.0, self.1.0);

        unsafe { vst1q_f32_x2(dest.as_mut_ptr(), stored) }
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        self
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline(always)]
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        Self(
            self.0.normalized_mul_add(other1.0, other2.0),
            self.1.normalized_mul_add(other1.1, other2.1),
        )
    }

    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        Self(
            self.0.normalized_mul_sub(other1.0, other2.0),
            self.1.normalized_mul_sub(other1.1, other2.1),
        )
    }

    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        Self(
            self.0.normalized_mul_mul_add(other1.0, other2.0, other3.0),
            self.1.normalized_mul_mul_add(other1.1, other2.1, other3.1),
        )
    }

    #[inline(always)]
    fn splat_color<T: ColorLike>(color: T) -> Self {
        Self::splat_4(color.to_rgbf32())
    }

    #[inline(always)]
    fn splat_alpha<T: ColorLike>(color: T) -> Self {
        Self::splat(color.to_rgbf32()[3])
    }

    #[inline(always)]
    fn min(mut self, other: Self) -> Self {
        self.0 = self.0.min(other.0);
        self.1 = self.1.min(other.1);

        self
    }

    #[inline(always)]
    fn max(mut self, other: Self) -> Self {
        self.0 = self.0.max(other.0);
        self.1 = self.1.max(other.1);

        self
    }

    #[inline(always)]
    fn from_float(f: &[Self::Float]) -> Self {
        f[0]
    }

    #[inline(always)]
    fn splat_4th_element(self) -> Self {
        Self(self.0.splat_4th_element(), self.1.splat_4th_element())
    }

    #[inline(always)]
    fn load_alphas_f32(src: &[f32]) -> Self {
        Self(
            f32x4::load_alphas_f32(&src[0..1]),
            f32x4::load_alphas_f32(&src[1..2]),
        )
    }

    fn load_f32_many(src: &[f32]) -> Self {
        todo!()
    }

    const IS_FLOAT: bool = true;
}

impl Widened<f32x8> for f32x8 {
    #[inline(always)]
    fn narrow(self) -> f32x8 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }

    #[inline(always)]
    fn clamp(self) -> Self {
        Self(self.0.clamp(), self.1.clamp())
    }
}

impl Float for f32x8 {
    type Integer = u32x8;

    #[inline(always)]
    fn sqrt(mut self) -> Self {
        self.0 = self.0.sqrt();
        self.1 = self.1.sqrt();

        self
    }

    #[inline(always)]
    fn powf(mut self, exponent: f32) -> Self {
        self.0 = self.0.powf(exponent);
        self.1 = self.1.powf(exponent);

        self
    }

    #[inline(always)]
    fn abs(mut self) -> Self {
        self.0 = self.0.abs();
        self.1 = self.1.abs();

        self
    }

    #[inline(always)]
    fn floor(mut self) -> Self {
        self.0 = self.0.floor();
        self.1 = self.1.floor();

        self
    }

    #[inline(always)]
    fn trunc(mut self) -> Self {
        self.0 = self.0.trunc();
        self.1 = self.1.trunc();

        self
    }

    #[inline(always)]
    fn reinterpret(mut self) -> Self::Integer {
        let a = self.0.reinterpret();
        let b = self.1.reinterpret();

        u32x8(a, b)
    }

    #[inline(always)]
    fn to_integer(mut self) -> Self::Integer {
        let a = self.0.to_integer();
        let b = self.1.to_integer();

        u32x8(a, b)
    }

    #[inline(always)]
    fn fract(mut self) -> Self {
        self.0 = self.0.fract();
        self.1 = self.1.fract();

        self
    }

    #[inline(always)]
    fn lt(mut self, other: Self) -> Self::Integer {
        let a = self.0.lt(other.0);
        let b = self.1.lt(other.1);

        u32x8(a, b)
    }

    #[inline(always)]
    fn leq(self, other: Self) -> Self::Integer {
        let a = self.0.leq(other.0);
        let b = self.1.leq(other.1);

        u32x8(a, b)
    }

    #[inline(always)]
    fn ne(mut self, other: Self) -> Self::Integer {
        let a = self.0.ne(other.0);
        let b = self.1.ne(other.1);

        u32x8(a, b)
    }

    #[inline(always)]
    fn if_then_else(mask: u32x8, if_: Self, else_: Self) -> Self {
        unsafe {
            let a = f32x4::if_then_else(mask.0, if_.0, else_.0);
            let b = f32x4::if_then_else(mask.1, if_.1, else_.1);

            Self(a, b)
        }
    }

    #[inline(always)]
    fn splat_x_col_pos(
        pos: f32,
        x_advance: f32,
        y_advance: f32,
    ) -> Self {
        let first_col = f32x4::splat_x_col_pos(pos, x_advance, y_advance);
        let second_col = f32x4::splat_x_col_pos(
            pos + x_advance,
            x_advance,
            y_advance,
        );

        f32x8(first_col, second_col)
    }

    #[inline(always)]
    fn splat_y_col_pos(
        pos: f32,
        x_advance: f32,
        y_advance: f32,
    ) -> Self {
        let first_col = f32x4::splat_x_col_pos(pos, x_advance, y_advance);
        let second_col = f32x4::splat_x_col_pos(
            pos + x_advance,
            x_advance,
            y_advance,
        );

        f32x8(first_col, second_col)
    }
}
