//! Test encoding with synthetic frames
//!
//! Run with: cargo run --example encode_test -p frame-core --features frame-core/encoding

use frame_core::encoder::{Encoder, EncoderConfig, VideoCodec, VideoFrame};
use std::path::Path;
use std::time::{Duration, Instant};

fn create_test_frame(width: u32, height: u32, frame_num: u32) -> VideoFrame {
    let size = (width * height * 4) as usize; // BGRA
    let mut data = vec![0u8; size];

    // Create a simple pattern that changes each frame
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;

            // Animated gradient
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = ((frame_num * 5) % 256) as u8;

            data[idx] = b; // B
            data[idx + 1] = g; // G
            data[idx + 2] = r; // R
            data[idx + 3] = 255; // A
        }
    }

    VideoFrame {
        data,
        width,
        height,
        timestamp: Duration::from_millis((frame_num as u64) * 33), // ~30 fps
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Frame Encoder Test");
    println!("==================\n");

    // Ensure ffmpeg is available
    println!("Checking ffmpeg...");
    Encoder::ensure_ffmpeg()?;
    println!("FFmpeg is ready!\n");

    // Configure encoder
    let config = EncoderConfig {
        video_codec: VideoCodec::H264,
        frame_rate: 30,
        hardware_accel: true,
        crf: 23,
        ..Default::default()
    };

    // Create encoder
    let mut encoder = Encoder::new(config)?;

    // Output path
    let output_path = Path::new("test_output.mp4");
    println!("Output: {:?}", output_path);

    // Initialize
    encoder.init(output_path)?;

    // Encode test frames
    let width = 1920;
    let height = 1080;
    let num_frames = 150; // 5 seconds at 30fps

    println!(
        "\nEncoding {} frames at {}x{}...",
        num_frames, width, height
    );

    let start = Instant::now();

    for i in 0..num_frames {
        let frame = create_test_frame(width, height, i);
        encoder.encode_frame(&frame)?;

        if (i + 1) % 30 == 0 {
            let elapsed = start.elapsed();
            let fps = (i + 1) as f64 / elapsed.as_secs_f64();
            println!("  Frame {}/{} ({:.1} fps)", i + 1, num_frames, fps);
        }
    }

    // Finalize
    println!("\nFinalizing...");
    encoder.finalize()?;

    let total_time = start.elapsed();
    let avg_fps = num_frames as f64 / total_time.as_secs_f64();

    println!("\n✓ Encoding complete!");
    println!("  Total time: {:.2}s", total_time.as_secs_f64());
    println!("  Average: {:.1} fps", avg_fps);
    println!("  Output: {:?}", output_path);

    // Check output file
    let metadata = std::fs::metadata(output_path)?;
    println!("  File size: {} KB", metadata.len() / 1024);

    Ok(())
}
