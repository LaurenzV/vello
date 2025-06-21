use crate::fine2::highp::gradient::SimdGradientKind;
use core::marker::PhantomData;
use vello_common::encode::LinearKind;
use vello_common::fearless_simd::{Simd, f32x4, f32x8, f32x16};

#[derive(Debug)]
pub struct SimdLinearKind<S: Simd> {
    phantom_data: PhantomData<S>,
}

impl<S: Simd> SimdLinearKind<S> {
    pub fn new(_: S, _: &LinearKind) -> Self {
        Self {
            phantom_data: PhantomData::default(),
        }
    }
}

impl<S: Simd> SimdGradientKind<S> for SimdLinearKind<S> {
    #[inline(always)]
    fn cur_pos(&self, x_pos: f32x4<S>, _: f32x4<S>) -> f32x4<S> {
        x_pos
    }
}
