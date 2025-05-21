use std::time::{Duration, Instant};
use vello_common::color::palette::css::ROYAL_BLUE;
use vello_common::encode::EncodedPaint;
use vello_common::paint::{Paint, PremulColor};
use vello_common::peniko::{BlendMode, Compose, Mix};
use vello_cpu::fine_experiments;
use vello_cpu::fine_experiments::SCRATCH_BUF_SIZE;
// use vello_cpu::fine2::{Fine};

fn main() {
    let paint = Paint::Solid(PremulColor::from_alpha_color(ROYAL_BLUE));
    let paints: Vec<EncodedPaint> = vec![];
    let a = PremulColor::from_alpha_color(ROYAL_BLUE)
        .as_premul_rgba8()
        .to_u8_array();
    let a_f32_4 = PremulColor::from_alpha_color(ROYAL_BLUE)
        .as_premul_f32()
        .components;
    let u8_arr = [a[0], a[1], a[2], a[3]];
    let u8_32 = u32::from_ne_bytes(u8_arr);
    let blend_mode = BlendMode::new(Mix::Normal, Compose::SrcOver);
    let mut blend_buf_u8 = vec![0u8; SCRATCH_BUF_SIZE];
    let blend_buf_f32 = vec![0.0f32; SCRATCH_BUF_SIZE];

    let max_duration = 3000;

    let mut count = 0;
    let iter_count = 100;
    let mut duration = Duration::default();
    // let mut fine = Fine::new();

    while duration.as_millis() < max_duration {
        let start = Instant::now();

        for _ in 0..100 {
            // fine.fill(&u8_arr);
            fine_experiments::opaque_u8(&mut blend_buf_u8, u8_32, u8_arr, 256);
            // fine_experiments::opaque_f32_2(&mut blend_buf_f32, &a_f32_4);
        }

        std::hint::black_box(&blend_buf_u8);

        count += 1;
        duration += start.elapsed();
    }

    let duration = duration.as_secs_f64() / (count * iter_count) as f64;

    if duration <= 1e-6 {
        println!("{}ns", duration * 1e+9);
    } else if duration <= 1e-3 {
        println!("{}μs", duration * 1e+6);
    } else if duration <= 1e-0 {
        println!("{}ms", duration * 1e+3);
    } else {
        println!("{}s", duration);
    }
}
