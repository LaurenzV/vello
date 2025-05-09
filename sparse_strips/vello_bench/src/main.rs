use std::time::{Duration, Instant};
use criterion::Bencher;
use vello_common::color::palette::css::ROYAL_BLUE;
use vello_common::encode::EncodedPaint;
use vello_common::paint::{Paint, PremulColor};
use vello_common::peniko::{BlendMode, Compose, Mix};
use vello_cpu::fine::{Fine, FineType};

fn main() {
    let paint = Paint::Solid(PremulColor::from_alpha_color(ROYAL_BLUE));
    let blend_mode = BlendMode::new(Mix::Normal, Compose::SrcOver);
    let encoded_paints = &[];
    let width = 256;
    
    let max_duration = 5000;
    
    let mut count = 0;
    let iter_count = 100;
    let mut duration = Duration::default();
    let mut fine = Fine::<u8>::new(256, 4);

    while duration.as_millis() < max_duration {
        let start = Instant::now();
        
        for _ in 0..iter_count {
            fine.fill(0, width, &paint, blend_mode, encoded_paints);
        }

        std::hint::black_box(&fine);
        count += 1;
        duration += start.elapsed();
    }
    
    let duration = duration.as_secs_f64() / (count * iter_count) as f64;
    
    if duration <= 1e-6 {
        println!("{}ns", duration * 1e+9);
    }   else if duration <= 1e-3 {
        println!("{}μs", duration * 1e+6);
    }   else if duration <= 1e-0 {
        println!("{}ms", duration * 1e+3);
    }   else {
        println!("{}s", duration);
    }
}