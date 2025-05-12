use crate::{Base, Narrowed, Widened};
use std::arch::aarch64::*;
use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub struct f32x4(float32x4_t);

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

impl Base for f32x4 {}

impl Narrowed<4, 1> for f32x4 {
    type Scalar = f32;
    type Widened = f32x4;

    #[inline(always)]
    fn load(src: &[f32; 4]) -> Self {
        unsafe { Self(vld1q_f32(src.as_ptr())) }
    }

    fn load_alphas(src: &[u8; 1]) -> Self {
        Self::from_normalized_u8(src[0])
    }

    #[inline(always)]
    fn load_4(src: &[f32; 4]) -> Self {
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
    fn store(self, dest: &mut [f32; 4]) {
        unsafe { vst1q_f32(dest.as_mut_ptr(), self.0) }
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        self
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }
}

impl Widened<4, 1, f32x4> for f32x4 {
    #[inline(always)]
    fn narrow(self) -> f32x4 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }
}

#[derive(Copy, Clone, Debug)]
pub struct f32x8(f32x4, f32x4);

impl Add for f32x8 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Mul for f32x8 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 * rhs.0;
        self.1 = self.1 * rhs.1;

        self
    }
}

impl Sub for f32x8 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 - rhs.0;
        self.1 = self.1 - rhs.1;

        self
    }
}

impl Base for f32x8 {}

impl Narrowed<8, 2> for f32x8 {
    type Scalar = f32;
    type Widened = Self;

    #[inline(always)]
    fn load(src: &[f32; 8]) -> Self {
        unsafe {
            let loaded = vld1q_f32_x2(src.as_ptr());

            Self(f32x4(loaded.0), f32x4(loaded.1))
        }
    }

    #[inline(always)]
    fn load_alphas(src: &[u8; 2]) -> Self {
        Self(
            f32x4::from_normalized_u8(src[0]),
            f32x4::from_normalized_u8(src[1]),
        )
    }

    #[inline(always)]
    fn load_4(src: &[f32; 4]) -> Self {
        unsafe {
            let v = vld1q_f32(src.as_ptr());

            Self(f32x4(v), f32x4(v))
        }
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe {
            let v = vdupq_n_f32(value);

            Self(f32x4(v), f32x4(v))
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32; 8]) {
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
}

impl Widened<8, 2, f32x8> for f32x8 {
    #[inline(always)]
    fn narrow(self) -> f32x8 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }
}

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

impl Base for u16x16 {}

impl Widened<16, 4, u8x16> for u16x16 {
    #[inline(always)]
    fn narrow(self) -> u8x16 {
        unsafe {
            let low = vmovn_u16(self.0.0);
            let high = vmovn_u16(self.0.1);

            u8x16(vcombine_u8(low, high))
        }
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        Self(uint16x8x2_t(div_255(self.0.0), div_255(self.0.1)))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u16x32(u16x16, u16x16);

impl Add for u16x32 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Mul for u16x32 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 * rhs.0;
        self.1 = self.1 * rhs.1;

        self
    }
}

impl Sub for u16x32 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 - rhs.0;
        self.1 = self.1 - rhs.1;

        self
    }
}

impl Base for u16x32 {}

impl Widened<32, 8, u8x32> for u16x32 {
    #[inline(always)]
    fn narrow(self) -> u8x32 {
        let first = self.0.narrow();
        let second = self.1.narrow();

        u8x32(first, second)
    }

    #[inline(always)]
    fn normalize(mut self) -> Self {
        self.0 = self.0.normalize();
        self.1 = self.1.normalize();

        self
    }
}

// #[derive(Copy, Clone, Debug)]
// pub struct u16x64(u16x32, u16x32);
//
// impl Add for u16x64 {
//     type Output = Self;
//
//     #[inline(always)]
//     fn add(mut self, rhs: Self) -> Self::Output {
//         unsafe {
//             self.0 = self.0 + rhs.0;
//             self.1 = self.1 + rhs.1;
//
//             self
//         }
//     }
// }
//
// impl Mul for u16x64 {
//     type Output = Self;
//
//     #[inline(always)]
//     fn mul(mut self, rhs: Self) -> Self::Output {
//         unsafe {
//             self.0 = self.0 * rhs.0;
//             self.1 = self.1 * rhs.1;
//
//             self
//         }
//     }
// }
//
// impl Sub for u16x64 {
//     type Output = Self;
//
//     #[inline(always)]
//     fn sub(mut self, rhs: Self) -> Self::Output {
//         unsafe {
//             self.0 = self.0 - rhs.0;
//             self.1 = self.1 - rhs.1;
//
//             self
//         }
//     }
// }
//
// impl Base for u16x64 {}

#[derive(Copy, Clone, Debug)]
pub struct u8x16(uint8x16_t);

impl Add for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_u8(self.0, rhs.0)) }
    }
}

impl Mul for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { Self(vmulq_u8(self.0, rhs.0)) }
    }
}

impl Sub for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self(vsubq_u8(self.0, rhs.0)) }
    }
}

impl Base for u8x16 {}

impl Narrowed<16, 4> for u8x16 {
    type Scalar = u8;
    type Widened = u16x16;

    #[inline(always)]
    fn load(src: &[u8; 16]) -> Self {
        unsafe { Self(vld1q_u8(src.as_ptr())) }
    }

    fn load_alphas(src: &[u8; 4]) -> Self {
        todo!()
    }

    #[inline(always)]
    fn load_4(src: &[u8; 4]) -> Self {
        unsafe {
            let loaded = vreinterpretq_u8_u32(vld1q_u32(src.as_ptr() as *const u32));
            Self(loaded)
        }
    }

    #[inline(always)]
    fn splat(value: u8) -> Self {
        unsafe {
            let loaded = vreinterpretq_u8_u32(vdupq_n_u32(u32::from_be_bytes([
                value, value, value, value,
            ])));
            Self(loaded)
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    fn store(self, dest: &mut [u8; 16]) {
        unsafe { vst1q_u8(dest.as_mut_ptr(), self.0) }
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        unsafe {
            let low = vget_low_u8(self.0);
            let high = vget_high_u8(self.0);

            u16x16(uint16x8x2_t(vmovl_u8(high), vmovl_u8(low)))
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u8x32(u8x16, u8x16);

impl Add for u8x32 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Mul for u8x32 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Sub for u8x32 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 - rhs.0;
        self.1 = self.1 - rhs.1;

        self
    }
}

impl Base for u8x32 {}

impl Narrowed<32, 8> for u8x32 {
    type Scalar = u8;
    type Widened = u16x32;

    #[inline(always)]
    fn load(src: &[u8; 32]) -> Self {
        unsafe {
            let loaded = vld1q_u8_x2(src.as_ptr());

            Self(u8x16(loaded.0), u8x16(loaded.1))
        }
    }

    fn load_alphas(src: &[u8; 8]) -> Self {
        todo!()
    }

    #[inline(always)]
    fn load_4(src: &[u8; 4]) -> Self {
        unsafe {
            let loaded = vreinterpretq_u8_u32(vld1q_u32(src.as_ptr() as *const u32));

            Self(u8x16(loaded), u8x16(loaded))
        }
    }

    #[inline(always)]
    fn splat(value: u8) -> Self {
        unsafe {
            let loaded = vreinterpretq_u8_u32(vdupq_n_u32(u32::from_be_bytes([
                value, value, value, value,
            ])));

            Self(u8x16(loaded), u8x16(loaded))
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    fn store(self, dest: &mut [u8; 32]) {
        let stored = uint8x16x2_t(self.0.0, self.1.0);
        unsafe { vst1q_u8_x2(dest.as_mut_ptr(), stored) }
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        let first = self.0.widen();
        let second = self.1.widen();

        u16x32(first, second)
    }
}

// #[derive(Copy, Clone, Debug)]
// pub struct u8x64(uint8x16x4_t);
//
// impl Add for u8x64 {
//     type Output = Self;
//
//     #[inline(always)]
//     fn add(mut self, rhs: Self) -> Self::Output {
//         unsafe {
//             self.0.0 = vaddq_u8(self.0.0, rhs.0.0);
//             self.0.1 = vaddq_u8(self.0.1, rhs.0.1);
//             self.0.2 = vaddq_u8(self.0.2, rhs.0.2);
//             self.0.3 = vaddq_u8(self.0.3, rhs.0.3);
//
//             self
//         }
//     }
// }
//
// impl Mul for u8x64 {
//     type Output = Self;
//
//     #[inline(always)]
//     fn mul(mut self, rhs: Self) -> Self::Output {
//         unsafe {
//             self.0.0 = vmulq_u8(self.0.0, rhs.0.0);
//             self.0.1 = vmulq_u8(self.0.1, rhs.0.1);
//             self.0.2 = vmulq_u8(self.0.2, rhs.0.2);
//             self.0.3 = vmulq_u8(self.0.3, rhs.0.3);
//
//             self
//         }
//     }
// }
//
// impl Sub for u8x64 {
//     type Output = Self;
//
//     #[inline(always)]
//     fn sub(mut self, rhs: Self) -> Self::Output {
//         unsafe {
//             self.0.0 = vsubq_u8(self.0.0, rhs.0.0);
//             self.0.1 = vsubq_u8(self.0.1, rhs.0.1);
//             self.0.2 = vsubq_u8(self.0.2, rhs.0.2);
//             self.0.3 = vsubq_u8(self.0.3, rhs.0.3);
//
//             self
//         }
//     }
// }
//
// impl Base for u8x64 {}
//
// impl Narrowed<64, u8> for u8x64 {
//     #[inline(always)]
//     fn load(src: &[u8; 64]) -> Self {
//         unsafe { Self(vld1q_u8_x4(src.as_ptr())) }
//     }
//
//     #[inline(always)]
//     fn load_4(src: &[u8; 4]) -> Self {
//         unsafe {
//             let loaded = vreinterpretq_u8_u32(vld1q_u32(src.as_ptr() as *const u32));
//             Self(uint8x16x4_t(
//                 loaded,
//                 vdupq_n_u8(0),
//                 vdupq_n_u8(0),
//                 vdupq_n_u8(0),
//             ))
//         }
//     }
//
//     #[inline(always)]
//     fn splat(value: u8) -> Self {
//         unsafe {
//             let loaded = vreinterpretq_u8_u32(vdupq_n_u32(u32::from_be_bytes([
//                 value, value, value, value,
//             ])));
//             Self(uint8x16x4_t(loaded, loaded, loaded, loaded))
//         }
//     }
//
//     #[inline(always)]
//     fn store(self, dest: &mut [u8; 64]) {
//         unsafe { vst1q_u8_x4(dest.as_mut_ptr(), self.0) }
//     }
// }

#[inline(always)]
fn div_255(input: uint16x8_t) -> uint16x8_t {
    unsafe {
        let p1 = vdupq_n_u16(255);
        let p2 = vaddq_u16(input, p1);
        vshrq_n_u16::<8>(p2)
    }
}
