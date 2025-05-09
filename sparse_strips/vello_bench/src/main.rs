use std::time::{Duration, Instant};
use criterion::Bencher;
use vello_common::color::palette::css::ROYAL_BLUE;
use vello_common::encode::EncodedPaint;
use vello_common::paint::{Paint, PremulColor};
use vello_common::peniko::{BlendMode, Compose, Mix};
use vello_cpu::fine2;
use vello_cpu::fine2::{SCRATCH_BUF_SIZE};
use vello_cpu::fine::ScratchBuf;

fn main() {
    let paint = PremulColor::from_alpha_color(ROYAL_BLUE).as_premul_rgba8().to_u8_array();
    let blend_mode = BlendMode::new(Mix::Normal, Compose::SrcOver);
    let mut blend_buf = [0u8; SCRATCH_BUF_SIZE];
    
    let max_duration = 3000;
    
    let mut count = 0;
    let iter_count = 100;
    let mut duration = Duration::default();
    // let mut fine = Fine::<f32>::new(256, 4);

    while duration.as_millis() < max_duration {
        let start = Instant::now();
        
        for _ in 0..100 {
            fine2::opaque(&mut blend_buf, &paint);
        }
        
        std::hint::black_box(&blend_buf);
        
        count += 1;
        duration += start.elapsed();
    }
    
    eprintln!("{:?}", duration);
    
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