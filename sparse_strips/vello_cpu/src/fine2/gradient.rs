// 
// use vello_common::encode::{EncodedGradient, GradientLike, GradientRange};
// use vello_common::kurbo::Point;
// use vello_simd::Type;
// use crate::fine2::{Painter, TILE_HEIGHT_COMPONENTS};
// 
// #[derive(Debug)]
// pub(crate) struct GradientFiller<'a, G: GradientLike> {
//     cur_pos: Point,
//     range_idx: usize,
//     gradient: &'a EncodedGradient,
//     kind: &'a G,
//     cur_range: &'a GradientRange,
// }
// 
// impl<'a, G: GradientLike> GradientFiller<'a, G> {
//     pub(crate) fn new(
//         gradient: &'a EncodedGradient,
//         kind: &'a G,
//         start_x: u16,
//         start_y: u16,
//     ) -> Self {
//         Self {
//             cur_pos: gradient.transform * Point::new(f64::from(start_x), f64::from(start_y)),
//             range_idx: 0,
//             cur_range: &gradient.ranges[0],
//             gradient,
//             kind,
//         }
//     }
// 
//     fn advance(&mut self, target_pos: f32) {
//         while target_pos > self.cur_range.x1 || target_pos < self.cur_range.x0 {
//             if self.range_idx == 0 {
//                 self.range_idx = self.gradient.ranges.len() - 1;
//             } else {
//                 self.range_idx -= 1;
//             }
// 
//             self.cur_range = &self.gradient.ranges[self.range_idx];
//         }
//     }
// 
//     pub(super) fn run<T: Type>(mut self, target: &mut [T::Scalar]) {
//         target
//             .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
//             .for_each(|column| {
//                 self.run_column(column);
//                 self.cur_pos += self.gradient.x_advance;
//             });
//     }
// 
//     fn run_column<T: Type>(&mut self, col: &mut [T::Scalar]) {
//         let pad = self.gradient.pad;
//         let extend = |val| extend(val, pad);
//         let mut pos = self.cur_pos;
// 
//         for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
//             let dist = extend(self.kind.cur_pos(&pos));
//             self.advance(dist);
//             let range = self.cur_range;
//             let c0 = range.c0.as_premul_f32().components;
// 
//             for (comp_idx, comp) in pixel.iter_mut().enumerate() {
//                 let factor = range.factors_f32[comp_idx] * (dist - range.x0);
// 
//                 *comp = F::from_normalized_f32(c0[comp_idx] + factor);
//             }
// 
//             pos += self.gradient.y_advance;
//         }
//     }
// 
// }
// 
// impl<F: Type, T: GradientLike> Painter<F> for GradientFiller<T> {
//     fn paint(self, target: &mut [F::Scalar]) {
//         self.run::<F>(target);
//     }
// }
// 
// pub(crate) fn extend(mut val: f32, pad: bool) -> f32 {
//     if pad {
//         val
//     } else {
//         while val < 0.0 {
//             val += 1.0;
//         }
// 
//         while val > 1.0 {
//             val -= 1.0;
//         }
// 
//         val
//     }
// }
