mod highp;
mod lowp;

use crate::peniko::{BlendMode, Compose, Mix};
use crate::region::Region;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::iter;
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
use crate::util::{BlendModeExt, InlineMapExt};
pub use highp::F32Kernel;
pub use lowp::U8Kernel;
use vello_common::fearless_simd::{
    Simd, SimdBase, SimdFloat, SimdInto, f32x4, f32x8, f32x16, u8x16, u8x32, u32x4, u32x8,
};

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

pub trait ShaderType<S: Simd>: Copy + Clone + Send + Sync {
    fn from_f32(simd: S, val: f32x16<S>) -> Self;
    fn from_u8(simd: S, val: u8x16<S>) -> Self;
}

impl<S: Simd> ShaderType<S> for f32x16<S> {
    #[inline(always)]
    fn from_f32(_: S, val: f32x16<S>) -> Self {
        val
    }

    #[inline(always)]
    fn from_u8(simd: S, val: u8x16<S>) -> Self {
        let converted: f32x16<_> = [
            val.val[0] as f32,
            val.val[1] as f32,
            val.val[2] as f32,
            val.val[3] as f32,
            val.val[4] as f32,
            val.val[5] as f32,
            val.val[6] as f32,
            val.val[7] as f32,
            val.val[8] as f32,
            val.val[9] as f32,
            val.val[10] as f32,
            val.val[11] as f32,
            val.val[12] as f32,
            val.val[13] as f32,
            val.val[14] as f32,
            val.val[15] as f32,
        ].simd_into(simd);
        
        converted * f32x16::splat(simd, 1.0 / 255.0)
    }
}

impl<S: Simd> ShaderType<S> for u8x16<S> {
    #[inline(always)]
    fn from_f32(simd: S, val: f32x16<S>) -> Self {
        let v1 = f32x16::splat(simd, 255.0);
        let v2 = f32x16::splat(simd, 0.5);
        let mulled = v2.madd(v1, val);

        // TODO: SIMDify
        [
            mulled.val[0] as u8,
            mulled.val[1] as u8,
            mulled.val[2] as u8,
            mulled.val[3] as u8,
            mulled.val[4] as u8,
            mulled.val[5] as u8,
            mulled.val[6] as u8,
            mulled.val[7] as u8,
            mulled.val[8] as u8,
            mulled.val[9] as u8,
            mulled.val[10] as u8,
            mulled.val[11] as u8,
            mulled.val[12] as u8,
            mulled.val[13] as u8,
            mulled.val[14] as u8,
            mulled.val[15] as u8,
        ].simd_into(simd)
    }

    #[inline(always)]
    fn from_u8(_: S, val: u8x16<S>) -> Self {
        val
    }
}

pub trait CompositeType<N: Numeric, S: Simd>: Copy + Clone + Send + Sync {
    const LENGTH: usize;

    fn from_slice(simd: S, slice: &[N]) -> Self;
    fn from_color(simd: S, color: [N; 4]) -> Self;
}

impl<S: Simd> CompositeType<f32, S> for f32x16<S> {
    const LENGTH: usize = 16;

    #[inline(always)]
    fn from_slice(simd: S, slice: &[f32]) -> Self {
        <f32x16<_> as SimdBase<_, _>>::from_slice(simd, slice)
    }

    #[inline(always)]
    fn from_color(simd: S, color: [f32; 4]) -> Self {
        f32x16::block_splat(f32x4::from_slice(simd, &color[..]))
    }
}

impl<S: Simd> CompositeType<u8, S> for u8x32<S> {
    const LENGTH: usize = 32;

    #[inline(always)]
    fn from_slice(simd: S, slice: &[u8]) -> Self {
        <u8x32<_> as SimdBase<_, _>>::from_slice(simd, slice)
    }

    #[inline(always)]
    fn from_color(simd: S, color: [u8; 4]) -> Self {
        u32x8::block_splat(u32x4::splat(simd, u32::from_ne_bytes(color))).reinterpret_u8()
    }
}

pub trait FineKernel<S: Simd>: Send + Sync + 'static {
    type Numeric: Numeric;
    type Composite: CompositeType<Self::Numeric, S>;
    type Shader: ShaderType<S>;

    fn extract_color(color: PremulColor) -> [Self::Numeric; 4];
    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]);
    fn copy_solid(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]);
    fn copy_f32_iter(simd: S, target: &mut [Self::Numeric], src: impl Iterator<Item = Self::Shader>);
    fn alpha_composite_solid(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]);
    fn alpha_composite_shader(simd: S, target: &mut [Self::Numeric], shader_src: &[Self::Numeric]);
    fn blend(
        simd: S,
        target: &mut [Self::Numeric],
        src: impl Iterator<Item = Self::Composite>,
        blend_mode: BlendMode,
    );
    fn alpha_composite_solid_with_alphas(
        simd: S,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
        alphas: &[u8],
    );
    fn alpha_composite_shader_with_alphas(
        simd: S,
        target: &mut [Self::Numeric],
        shader_src: &[Self::Numeric],
        alphas: &[u8],
    );
    fn blend_with_alphas(
        simd: S,
        target: &mut [Self::Numeric],
        src: impl Iterator<Item = Self::Composite>,
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

        T::copy_solid(self.simd, blend_buf, converted_color);
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
                    .push([T::Numeric::ZERO; SCRATCH_BUF_SIZE]);
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
        let default_blend = blend_mode.is_default();

        match fill {
            Paint::Solid(color) => {
                let color = T::extract_color(*color);

                // If color is completely opaque, we can just directly override
                // the blend buffer.
                if color[3] == T::Numeric::ONE && default_blend {
                    T::copy_solid(self.simd, blend_buf, color);

                    return;
                }

                if default_blend {
                    T::alpha_composite_solid(self.simd, blend_buf, color);
                } else {
                    T::blend(
                        self.simd,
                        blend_buf,
                        iter::repeat(T::Composite::from_color(self.simd, color)),
                        blend_mode,
                    );
                }
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
                    default_blend: bool,
                    blend_mode: BlendMode,
                    filler: impl Iterator<Item = T::Shader>,
                ) {
                    if has_opacities {
                        T::copy_f32_iter(simd, color_buf, filler);

                        if default_blend {
                            T::alpha_composite_shader(simd, blend_buf, color_buf);
                        } else {
                            T::blend(
                                simd,
                                blend_buf,
                                color_buf
                                    .chunks_exact(T::Composite::LENGTH)
                                    .map(|s| T::Composite::from_slice(simd, s)),
                                blend_mode,
                            );
                        }
                    } else {
                        // Similarly to solid colors we can just override the previous values
                        // if all colors in the gradient are fully opaque.
                        T::copy_f32_iter(simd, blend_buf, filler);
                    }
                }

                match encoded_paint {
                    EncodedPaint::BlurredRoundedRect(b) => {
                        let filler = BlurredRoundedRectFiller::new(self.simd, b, start_x, start_y);

                        fill_complex_paint::<S, T>(
                            self.simd,
                            color_buf,
                            blend_buf,
                            true,
                            default_blend,
                            blend_mode,
                            filler.map(|i| T::Shader::from_f32(self.simd, i)),
                        );
                    }
                    EncodedPaint::Gradient(g) => match &g.kind {
                        EncodedKind::Linear(l) => {
                            let filler: GradientFiller<'_, S, SimdLinearKind<S>> =
                                GradientFiller::new(
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
                                default_blend,
                                blend_mode,
                                filler.inline_map(|i| T::Shader::from_f32(self.simd, i)),
                            );
                        }
                        EncodedKind::Sweep(s) => {
                            let filler: GradientFiller<'_, S, SimdSweepKind<S>> =
                                GradientFiller::new(
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
                                default_blend,
                                blend_mode,
                                filler.inline_map(|i| T::Shader::from_f32(self.simd, i)),
                            );
                        }
                        EncodedKind::Radial(r) => {
                            let filler: GradientFiller<'_, S, SimdRadialKind<S>> =
                                GradientFiller::new(
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
                                default_blend,
                                blend_mode,
                                filler.inline_map(|i| T::Shader::from_f32(self.simd, i)),
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
        let default_blend = blend_mode.is_default();

        match fill {
            Paint::Solid(color) => {
                let color = T::extract_color(*color);

                if default_blend {
                    T::alpha_composite_solid_with_alphas(self.simd, blend_buf, color, alphas);
                } else {
                    T::blend_with_alphas(
                        self.simd,
                        blend_buf,
                        iter::repeat(T::Composite::from_color(self.simd, color)),
                        blend_mode,
                        alphas,
                    );
                }
            }
            Paint::Indexed(paint) => {
                fn fill_complex_paint<S: Simd, T: FineKernel<S>>(
                    simd: S,
                    color_buf: &mut [T::Numeric],
                    blend_buf: &mut [T::Numeric],
                    default_blend: bool,
                    blend_mode: BlendMode,
                    filler: impl Iterator<Item = T::Shader>,
                    alphas: &[u8],
                ) {
                    T::copy_f32_iter(simd, color_buf, filler);

                    if default_blend {
                        T::alpha_composite_shader_with_alphas(simd, blend_buf, color_buf, alphas);
                    } else {
                        T::blend_with_alphas(
                            simd,
                            blend_buf,
                            color_buf
                                .chunks_exact(T::Composite::LENGTH)
                                .map(|s| T::Composite::from_slice(simd, s)),
                            blend_mode,
                            alphas,
                        );
                    }
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
                            self.simd,
                            color_buf,
                            blend_buf,
                            default_blend,
                            blend_mode,
                            filler.map(|i| T::Shader::from_f32(self.simd, i)),
                            alphas,
                        );
                    }
                    EncodedPaint::Gradient(g) => match &g.kind {
                        EncodedKind::Linear(l) => {
                            let filler: GradientFiller<'_, S, SimdLinearKind<S>> =
                                GradientFiller::new(
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
                                default_blend,
                                blend_mode,
                                filler.inline_map(|i| T::Shader::from_f32(self.simd, i)),
                                alphas,
                            );
                        }
                        EncodedKind::Sweep(s) => {
                            let filler: GradientFiller<'_, S, SimdSweepKind<S>> =
                                GradientFiller::new(
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
                                default_blend,
                                blend_mode,
                                filler.inline_map(|i| T::Shader::from_f32(self.simd, i)),
                                alphas,
                            );
                        }
                        EncodedKind::Radial(r) => {
                            let filler: GradientFiller<'_, S, SimdRadialKind<S>> =
                                GradientFiller::new(
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
                                default_blend,
                                blend_mode,
                                filler.inline_map(|i| T::Shader::from_f32(self.simd, i)),
                                alphas,
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

        T::blend(
            self.simd,
            target_buffer,
            source_buffer
                .chunks_exact(T::Composite::LENGTH)
                .map(|s| T::Composite::from_slice(self.simd, s)),
            blend_mode,
        );
    }

    fn clip_fill(&mut self, x: usize, width: usize) {
        let (source_buffer, rest) = self.blend_buf.split_last_mut().unwrap();
        let target_buffer = rest.last_mut().unwrap();

        let source_buffer =
            &mut source_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let target_buffer =
            &mut target_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        T::alpha_composite_shader(self.simd, target_buffer, source_buffer);
    }

    fn clip_strip(&mut self, x: usize, width: usize, alphas: &[u8]) {
        let (source_buffer, rest) = self.blend_buf.split_last_mut().unwrap();
        let target_buffer = rest.last_mut().unwrap();

        let source_buffer =
            &mut source_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let target_buffer =
            &mut target_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        T::alpha_composite_shader_with_alphas(self.simd, target_buffer, source_buffer, alphas);
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
        let zip_low = self.zip_high(self);
        zip_low.zip_high(zip_low)
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
