use std::io::Cursor;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use smallvec::smallvec;
use vello_common::color::{AlphaColor, DynamicColor};
use vello_common::kurbo::{Point, Shape};
use vello_common::peniko::{ColorStop, ColorStops, Gradient};
use vello_common::pixmap::Pixmap;
use vello_cpu::{RenderContext, RenderMode};
use vello_cpu::kurbo::Rect;
use vello_cpu::peniko::GradientKind::Radial;

fn main() {
    let mut ctx = RenderContext::new(100, 100);
    let rect = &Rect::new(0.0, 0.0, 100.0, 100.0);

    let gradient = Gradient {
        kind: Radial {
            start_center: Point::new(0.0, 50.0),
            start_radius: 0.0,
            end_center: Point::new(50.0, 50.0),
            end_radius: 100.0,
        },
        stops: ColorStops(smallvec![
            ColorStop {
                offset: 0.0,
                color: DynamicColor::from_alpha_color(AlphaColor::from_rgba8(50, 127, 150, 200)),
                },
            ColorStop {
                offset: 0.25,
                color: DynamicColor::from_alpha_color(AlphaColor::from_rgba8(50, 127, 150, 200)),
                },
            ColorStop {
                offset: 0.5,
                color: DynamicColor::from_alpha_color(AlphaColor::from_rgba8(0, 255, 0, 200)),
                },
            ColorStop {
                offset: 0.75,
                color: DynamicColor::from_alpha_color(AlphaColor::from_rgba8(220, 140, 75, 180)),
            },
            ColorStop {
                offset: 1.0,
                color: DynamicColor::from_alpha_color(AlphaColor::from_rgba8(220, 140, 75, 180)),
            },
        ]),
        ..Default::default()
    };

    ctx.set_paint(gradient);
    ctx.fill_rect(rect);
    
    let mut pix = Pixmap::new(100, 100);
    
    ctx.render_to_pixmap(&mut pix, RenderMode::OptimizeQuality);

    let img_buf = pix.take_unpremultiplied();

    let mut png_data = Vec::new();
    let cursor = Cursor::new(&mut png_data);
    let encoder = PngEncoder::new(cursor);
    encoder
        .write_image(
            bytemuck::cast_slice(&img_buf),
            100,
            100,
            ExtendedColorType::Rgba8,
        )
        .expect("Failed to encode image");
    
    std::fs::write("out.png", png_data);
}
