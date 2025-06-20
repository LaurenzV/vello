use vello_common::fearless_simd::{f32x8, Simd};

mod linear;

trait SimdGradientKind<S: Simd> {
    fn cur_pos(&self, x_pos: f32x8<S>, y_pos: f32x8<S>) -> f32x8<S>;
    fn has_undefined(&self) -> bool {
        false
    }
}