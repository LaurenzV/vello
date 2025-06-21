mod highp;
mod lowp;

use crate::peniko::{BlendMode, Compose, Mix};
use crate::region::Region;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;
use vello_common::coarse::{Cmd, WideTile};
use vello_common::encode::{EncodedKind, EncodedPaint};
use vello_common::paint::{Paint, PremulColor};
use vello_common::tile::Tile;

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TILE_HEIGHT_COMPONENTS: usize = Tile::HEIGHT as usize * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize =
    WideTile::WIDTH as usize * Tile::HEIGHT as usize * COLOR_COMPONENTS;

use crate::fine2::highp::gradient::GradientFiller;
use crate::fine2::highp::gradient::linear::SimdLinearKind;
use crate::fine2::highp::gradient::radial::SimdRadialKind;
use crate::fine2::highp::gradient::sweep::SimdSweepKind;
use crate::fine2::highp::rounded_blurred_rect::BlurredRoundedRectFiller;
pub use highp::F32Kernel;
pub use lowp::U8Kernel;
use vello_common::fearless_simd::{
    Simd, SimdBase, SimdFloat, SimdInto, f32x4, f32x8, f32x16, u8x16, u8x32,
};
use crate::util::BlendModeExt;

pub type ScratchBuf<F> = [F; SCRATCH_BUF_SIZE];

pub trait Numeric: Copy + Default + Clone + Debug + PartialEq + Send + Sync + 'static {
    const ZERO: Self;
    const ONE: Self;
}

impl Numeric for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
}

impl Numeric for u8 {
    const ZERO: Self = 0;
    const ONE: Self = 255;
}

pub trait CompositeType<N: Numeric, S: Simd>: Copy + Clone + Send + Sync {
    const LENGTH: usize;
    
    fn from_slice(&self, simd: S, slice: &[N]) -> Self;
}

impl<S: Simd> CompositeType<f32, S> for f32x16<S> {
    const LENGTH: usize = 16;

    #[inline(always)]
    fn from_slice(&self, simd: S, slice: &[f32]) -> Self {
        <f32x16<_> as SimdBase<_, _>>::from_slice(simd, slice)
    }
}

impl<S: Simd> CompositeType<u8, S> for u8x32<S> {
    const LENGTH: usize = 32;

    #[inline(always)]
    fn from_slice(&self, simd: S, slice: &[u8]) -> Self {
        <u8x32<_> as SimdBase<_, _>>::from_slice(simd, slice)
    }
}

pub trait FineKernel<S: Simd>: Send + Sync + 'static {
    type Numeric: Numeric;
    type Composite: CompositeType<Self::Numeric, S>;

    fn extract_color(color: PremulColor) -> [Self::Numeric; 4];
    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]);
    fn fill_buf_solid(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]);
    fn fill_buf_arbitrary(
        simd: S,
        target: &mut [Self::Numeric],
        src: impl Iterator<Item = f32x16<S>>,
    );
    fn composite_solid(
        simd: S,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
        blend_mode: BlendMode,
    );
    fn composite_shader(
        simd: S,
        target: &mut [Self::Numeric],
        shader_src: &[Self::Numeric],
        blend_mode: BlendMode,
    );
    fn alpha_composite_solid(
        simd: S,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
        blend_mode: BlendMode,
        alphas: &[u8],
    );
    fn alpha_composite_shader(
        simd: S,
        target: &mut [Self::Numeric],
        shader_src: &[Self::Numeric],
        blend_mode: BlendMode,
        alphas: &[u8],
    );
}

#[derive(Debug)]
pub struct Fine<S: Simd, T: FineKernel<S>> {
    pub(crate) wide_coords: (u16, u16),
    pub(crate) blend_buf: Vec<ScratchBuf<T::Numeric>>,
    pub(crate) paint_buf: ScratchBuf<T::Numeric>,
    pub(crate) simd: S,
}

impl<S: Simd, T: FineKernel<S>> Fine<S, T> {
    pub fn new(simd: S) -> Self {
        Self {
            simd,
            wide_coords: (0, 0),
            blend_buf: vec![[T::Numeric::ZERO; SCRATCH_BUF_SIZE]],
            paint_buf: [T::Numeric::ZERO; SCRATCH_BUF_SIZE],
        }
    }

    pub fn set_coords(&mut self, x: u16, y: u16) {
        self.wide_coords = (x, y);
    }

    pub fn clear(&mut self, premul_color: PremulColor) {
        let converted_color = T::extract_color(premul_color);
        let blend_buf = self.blend_buf.last_mut().unwrap();

        T::fill_buf_solid(self.simd, blend_buf, converted_color);
    }

    pub fn pack(&self, region: &mut Region<'_>) {
        let blend_buf = self.blend_buf.last().unwrap();

        T::pack(region, blend_buf);
    }

    pub(crate) fn run_cmd(&mut self, cmd: &Cmd, alphas: &[u8], paints: &[EncodedPaint]) {
        match cmd {
            Cmd::Fill(f) => {
                self.fill(
                    usize::from(f.x),
                    usize::from(f.width),
                    &f.paint,
                    f.blend_mode
                        .unwrap_or(BlendMode::new(Mix::Normal, Compose::SrcOver)),
                    paints,
                );
            }
            Cmd::AlphaFill(s) => {
                let a_slice = &alphas[s.alpha_idx..];
                self.strip(
                    usize::from(s.x),
                    usize::from(s.width),
                    a_slice,
                    &s.paint,
                    s.blend_mode
                        .unwrap_or(BlendMode::new(Mix::Normal, Compose::SrcOver)),
                    paints,
                );
            }
            Cmd::PushBuf => {
                self.blend_buf
                    .push([T::Numeric::ZERO; crate::fine::SCRATCH_BUF_SIZE]);
            }
            Cmd::PopBuf => {
                self.blend_buf.pop();
            }
            Cmd::ClipFill(cf) => {
                self.clip_fill(cf.x as usize, cf.width as usize);
            }
            Cmd::ClipStrip(cs) => {
                let aslice = &alphas[cs.alpha_idx..];
                self.clip_strip(cs.x as usize, cs.width as usize, aslice);
            }
            Cmd::Blend(b) => self.apply_blend(*b),
            _ => unimplemented!(),
        }
    }

    #[inline(always)]
    /// Fill at a given x and with a width using the given paint.
    pub fn fill(
        &mut self,
        x: usize,
        width: usize,
        fill: &Paint,
        blend_mode: BlendMode,
        encoded_paints: &[EncodedPaint],
    ) {
        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];

        match fill {
            Paint::Solid(color) => {
                let color = T::extract_color(*color);

                // If color is completely opaque, we can just directly override
                // the blend buffer.
                if color[3] == T::Numeric::ONE && blend_mode.is_default() {
                    T::fill_buf_solid(self.simd, blend_buf, color);

                    return;
                }

                T::composite_solid(self.simd, blend_buf, color, blend_mode);
            }
            Paint::Indexed(paint) => {
                let color_buf = &mut self.paint_buf[x * TILE_HEIGHT_COMPONENTS..]
                    [..TILE_HEIGHT_COMPONENTS * width];

                let encoded_paint = &encoded_paints[paint.index()];

                let start_x = self.wide_coords.0 * WideTile::WIDTH + x as u16;
                let start_y = self.wide_coords.1 * Tile::HEIGHT;

                fn fill_complex_paint<S: Simd, T: FineKernel<S>>(
                    simd: S,
                    color_buf: &mut [T::Numeric],
                    blend_buf: &mut [T::Numeric],
                    has_opacities: bool,
                    blend_mode: BlendMode,
                    filler: impl Iterator<Item = f32x16<S>>,
                ) {
                    if has_opacities {
                        T::fill_buf_arbitrary(simd, color_buf, filler);

                        T::composite_shader(simd, blend_buf, color_buf, blend_mode);
                    } else {
                        // Similarly to solid colors we can just override the previous values
                        // if all colors in the gradient are fully opaque.
                        T::fill_buf_arbitrary(simd, blend_buf, filler);
                    }
                }

                match encoded_paint {
                    EncodedPaint::BlurredRoundedRect(b) => {
                        let filler = BlurredRoundedRectFiller::new(self.simd, b, start_x, start_y);

                        fill_complex_paint::<S, T>(
                            self.simd, color_buf, blend_buf, true, blend_mode, filler,
                        );
                    }
                    EncodedPaint::Gradient(g) => match &g.kind {
                        EncodedKind::Linear(l) => {
                            let filler: GradientFiller<'_, S, SimdLinearKind<S>> = GradientFiller::new(
                                self.simd,
                                g,
                                SimdLinearKind::new(self.simd, l),
                                start_x,
                                start_y,
                            );

                            fill_complex_paint::<S, T>(
                                self.simd,
                                color_buf,
                                blend_buf,
                                g.has_opacities,
                                blend_mode,
                                filler,
                            );
                        }
                        EncodedKind::Sweep(s) => {
                            let filler: GradientFiller<'_, S, SimdSweepKind<S>> = GradientFiller::new(
                                self.simd,
                                g,
                                SimdSweepKind::new(self.simd, s),
                                start_x,
                                start_y,
                            );

                            fill_complex_paint::<S, T>(
                                self.simd,
                                color_buf,
                                blend_buf,
                                g.has_opacities,
                                blend_mode,
                                filler,
                            );
                        }
                        EncodedKind::Radial(r) => {
                            let filler: GradientFiller<'_, S, SimdRadialKind<S>> = GradientFiller::new(
                                self.simd,
                                g,
                                SimdRadialKind::new(self.simd, r),
                                start_x,
                                start_y,
                            );

                            fill_complex_paint::<S, T>(
                                self.simd,
                                color_buf,
                                blend_buf,
                                g.has_opacities,
                                blend_mode,
                                filler,
                            );
                        }
                    },
                    _ => unimplemented!(),
                }
            }
        }
    }

    /// Strip at a given x and with a width using the given paint and alpha values.
    #[inline(always)]
    pub fn strip(
        &mut self,
        x: usize,
        width: usize,
        alphas: &[u8],
        fill: &Paint,
        blend_mode: BlendMode,
        encoded_paints: &[EncodedPaint],
    ) {
        debug_assert!(
            alphas.len() >= width,
            "alpha buffer doesn't contain sufficient elements"
        );

        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];

        match fill {
            Paint::Solid(color) => {
                T::alpha_composite_solid(
                    self.simd,
                    blend_buf,
                    T::extract_color(*color),
                    blend_mode,
                    alphas,
                );
            }
            Paint::Indexed(paint) => {
                fn fill_complex_paint<S: Simd, T: FineKernel<S>>(
                    simd: S,
                    color_buf: &mut [T::Numeric],
                    blend_buf: &mut [T::Numeric],
                    blend_mode: BlendMode,
                    filler: impl Iterator<Item = f32x16<S>>,
                    alphas: &[u8],
                ) {
                    T::fill_buf_arbitrary(simd, color_buf, filler);

                    T::alpha_composite_shader(simd, blend_buf, color_buf, blend_mode, alphas);
                }

                let color_buf = &mut self.paint_buf[x * TILE_HEIGHT_COMPONENTS..]
                    [..TILE_HEIGHT_COMPONENTS * width];

                let encoded_paint = &encoded_paints[paint.index()];

                let start_x = self.wide_coords.0 * WideTile::WIDTH + x as u16;
                let start_y = self.wide_coords.1 * Tile::HEIGHT;

                match encoded_paint {
                    EncodedPaint::BlurredRoundedRect(b) => {
                        let filler = BlurredRoundedRectFiller::new(self.simd, b, start_x, start_y);

                        fill_complex_paint::<S, T>(
                            self.simd, color_buf, blend_buf, blend_mode, filler, alphas,
                        );
                    }
                    EncodedPaint::Gradient(g) => match &g.kind {
                        EncodedKind::Linear(l) => {
                            let filler: GradientFiller<'_, S, SimdLinearKind<S>> = GradientFiller::new(
                                self.simd,
                                g,
                                SimdLinearKind::new(self.simd, l),
                                start_x,
                                start_y,
                            );

                            fill_complex_paint::<S, T>(
                                self.simd, color_buf, blend_buf, blend_mode, filler, alphas,
                            );
                        }
                        EncodedKind::Sweep(s) => {
                            let filler: GradientFiller<'_, S, SimdSweepKind<S>> = GradientFiller::new(
                                self.simd,
                                g,
                                SimdSweepKind::new(self.simd, s),
                                start_x,
                                start_y,
                            );

                            fill_complex_paint::<S, T>(
                                self.simd, color_buf, blend_buf, blend_mode, filler, alphas,
                            );
                        }
                        EncodedKind::Radial(r) => {
                            let filler: GradientFiller<'_, S, SimdRadialKind<S>> = GradientFiller::new(
                                self.simd,
                                g,
                                SimdRadialKind::new(self.simd, r),
                                start_x,
                                start_y,
                            );

                            fill_complex_paint::<S, T>(
                                self.simd, color_buf, blend_buf, blend_mode, filler, alphas,
                            );
                        }
                    },
                    _ => unimplemented!(),
                }
            }
        }
    }

    fn apply_blend(&mut self, blend_mode: BlendMode) {
        let (source_buffer, rest) = self.blend_buf.split_last_mut().unwrap();
        let target_buffer = rest.last_mut().unwrap();

        T::composite_shader(self.simd, target_buffer, source_buffer, blend_mode);
    }

    fn clip_fill(&mut self, x: usize, width: usize) {
        let (source_buffer, rest) = self.blend_buf.split_last_mut().unwrap();
        let target_buffer = rest.last_mut().unwrap();

        let source_buffer =
            &mut source_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let target_buffer =
            &mut target_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        T::composite_shader(
            self.simd,
            target_buffer,
            source_buffer,
            BlendMode::new(Mix::Normal, Compose::SrcOver),
        );
    }

    fn clip_strip(&mut self, x: usize, width: usize, alphas: &[u8]) {
        let (source_buffer, rest) = self.blend_buf.split_last_mut().unwrap();
        let target_buffer = rest.last_mut().unwrap();

        let source_buffer =
            &mut source_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let target_buffer =
            &mut target_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        T::alpha_composite_shader(
            self.simd,
            target_buffer,
            source_buffer,
            BlendMode::new(Mix::Normal, Compose::SrcOver),
            alphas,
        );
    }
}

pub trait PosExt<S: Simd> {
    fn splat_col_pos(simd: S, pos: f32, x_advance: f32, y_advance: f32) -> Self;
}

impl<S: Simd> PosExt<S> for f32x4<S> {
    #[inline(always)]
    fn splat_col_pos(simd: S, pos: f32, _: f32, y_advance: f32) -> Self {
        let column_mask: f32x4<_> = [0.0, 1.0, 2.0, 3.0].simd_into(simd);

        f32x4::splat(simd, pos).madd(column_mask, f32x4::splat(simd, y_advance))
    }
}

impl<S: Simd> PosExt<S> for f32x8<S> {
    #[inline(always)]
    fn splat_col_pos(simd: S, pos: f32, x_advance: f32, y_advance: f32) -> Self {
        simd.combine_f32x4(
            f32x4::splat_col_pos(simd, pos, x_advance, y_advance),
            f32x4::splat_col_pos(simd, pos + x_advance, x_advance, y_advance),
        )
    }
}

pub trait Splat4thExt<S> {
    fn splat_4th(self) -> Self;
}

impl<S: Simd> Splat4thExt<S> for f32x4<S> {
    #[inline(always)]
    fn splat_4th(self) -> Self {
        let zip1 = self.zip2(self);
        zip1.zip2(zip1)
    }
}

impl<S: Simd> Splat4thExt<S> for f32x8<S> {
    #[inline(always)]
    fn splat_4th(self) -> Self {
        let (mut p1, mut p2) = self.simd.split_f32x8(self);
        p1 = p1.splat_4th();
        p2 = p2.splat_4th();

        self.simd.combine_f32x4(p1, p2)
    }
}

impl<S: Simd> Splat4thExt<S> for f32x16<S> {
    #[inline(always)]
    fn splat_4th(self) -> Self {
        let (mut p1, mut p2) = self.simd.split_f32x16(self);
        p1 = p1.splat_4th();
        p2 = p2.splat_4th();

        self.simd.combine_f32x8(p1, p2)
    }
}

impl<S: Simd> Splat4thExt<S> for u8x16<S> {
    #[inline(always)]
    fn splat_4th(self) -> Self {
        // TODO: SIMDify
        u8x16 {
            val: [
                self.val[3],
                self.val[3],
                self.val[3],
                self.val[3],
                self.val[7],
                self.val[7],
                self.val[7],
                self.val[7],
                self.val[11],
                self.val[11],
                self.val[11],
                self.val[11],
                self.val[15],
                self.val[15],
                self.val[15],
                self.val[15],
            ],
            simd: self.simd,
        }
    }
}

impl<S: Simd> Splat4thExt<S> for u8x32<S> {
    #[inline(always)]
    fn splat_4th(self) -> Self {
        let (mut p1, mut p2) = self.simd.split_u8x32(self);
        p1 = p1.splat_4th();
        p2 = p2.splat_4th();

        self.simd.combine_u8x16(p1, p2)
    }
}
