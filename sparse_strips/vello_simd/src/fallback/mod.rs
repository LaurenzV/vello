mod f32x16;
mod f32x4;
mod f32x8;
mod u32x16;
mod u32x4;
mod u32x8;
mod u8x16;
mod u8x32;
mod u8x64;

use crate::{Base, NumberKind, Simd};

#[derive(Copy, Clone, Debug)]
pub struct Fallback;

impl Simd for Fallback {
    type Integer = u8x64::u8x64;
    type Float = f32x16::f32x16;
}

impl NumberKind for f32 {
    const ZERO: Self = 0.0;
    const MID: Self = 0.5;
    const ONE: Self = 1.0;

    #[inline(always)]
    fn to_rgba8(src: &[Self]) -> [u8; 4] {
        [
            (src[0] * 255.0 + 0.5) as u8,
            (src[1] * 255.0 + 0.5) as u8,
            (src[2] * 255.0 + 0.5) as u8,
            (src[3] * 255.0 + 0.5) as u8,
        ]
    }

    fn from_rgba8(src: &[u8; 4]) -> [Self; 4] {
        [
            src[0] as f32 * (1.0 / 255.0),
            src[1] as f32 * (1.0 / 255.0),
            src[2] as f32 * (1.0 / 255.0),
            src[3] as f32 * (1.0 / 255.0),
        ]
    }

    fn from_normalized_f32(val: f32) -> Self {
        val
    }

    fn normalized_mul(&self, other: Self) -> Self {
        self * other
    }
}

impl NumberKind for u8 {
    const ZERO: Self = 0;
    const MID: Self = 127;
    const ONE: Self = 255;

    #[inline(always)]
    fn to_rgba8(src: &[Self]) -> [u8; 4] {
        [src[0], src[1], src[2], src[3]]
    }

    fn from_rgba8(src: &[u8; 4]) -> [Self; 4] {
        *src
    }

    fn from_normalized_f32(val: f32) -> Self {
        (val * 255.0 + 0.5) as u8
    }

    fn normalized_mul(&self, other: Self) -> Self {
        div_255(*self as u16 * other as u16) as u8
    }
}

impl Base for f32 {}
impl Base for u8 {}
impl Base for u16 {}

/// Perform an approximate division by 255.
///
/// There are three reasons for having this method.
/// 1) Divisions are slower than shifting + adding, and the compiler does not seem to replace
///    divisions by 255 with an equivalent (this was verified by benchmarking; doing / 255 was
///    significantly slower).
/// 2) Integer divisions are usually not available in SIMD, so this provides a good baseline
///    implementation.
/// 3) There are two options for performing the division: One is to perform the division
///    in a way that completely preserves the rounding semantics of a integer division by
///    255. This could be achieved using the implementation `(val + 1 + (val >> 8)) >> 8`.
///    The second approach (used here) has slightly different rounding behavior to a
///    normal division by 255, but is much faster (see <https://github.com/linebender/vello/issues/904>)
///    and therefore preferable for the high-performance pipeline.
///
/// Four properties worth mentioning:
/// - This actually calculates the ceiling of `val / 256`.
/// - Within the allowed range for `val`, rounding errors do not appear for values divisible by 255, i.e. any call `div_255(val * 255)` will always yield `val`.
/// - If there is a discrepancy, this division will always yield a value 1 higher than the original.
/// - This holds for values of `val` up to and including `65279`. You should not call this function with higher values.
#[inline(always)]
pub(crate) const fn div_255(val: u16) -> u16 {
    debug_assert!(
        val < 65280,
        "the properties of `div_255` do not hold for values of `65280` or greater"
    );
    (val + 255) >> 8
}

#[cfg(test)]
mod tests {
    use crate::fallback::div_255;

    #[test]
    fn division() {
        for i in 0_u16..=(255 * 255) {
            let expected = i / 255;
            let actual = div_255(i);

            let diff = expected.abs_diff(actual);

            // Rounding error shouldn't be higher than 1.
            assert!(diff <= 1);

            if i % 255 == 0 {
                // Division should be accurate for multiples of 255.
                assert_eq!(diff, 0);
            }
        }
    }
}
