use api_s3::models::device::{DeviceConfig, DeviceTarget};
use api_s3::services::image_processor;
use image::GenericImageView;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_path = "bench_assets/benchmark.jpg";
    let image_data = fs::read(input_path)?;
    let original_size = image_data.len();

    let src_image = image::load_from_memory(&image_data)?;
    let (w, h) = src_image.dimensions();

    println!("=============================================");
    println!("   BENCHMARK COMPLET : OPTIMISATION S3-API  ");
    println!("=============================================");
    println!("Image source : {} ({}x{})", input_path, w, h);
    println!("Taille originale : {} Ko", original_size / 1024);
    println!("---------------------------------------------");

    let tests = vec![
        ("PHONE  (720x1600)", DeviceTarget::Phone, 720, 1600),
        ("TABLET (1200x1920)", DeviceTarget::Tablet, 1200, 1920),
        ("DESKTOP (1920x1080)", DeviceTarget::Desktop, 1920, 1080),
        ("BYPASS (4K screen)", DeviceTarget::Desktop, 3840, 2160),
    ];

    for (label, target, tw, th) in tests {
        let config = DeviceConfig {
            target,
            width: tw,
            height: th,
            scale: None,
        };

        let start = std::time::Instant::now();
        let processed = image_processor::process_image(image_data.clone(), config).await?;
        let duration = start.elapsed();

        let size = processed.len();
        let gain = (original_size as i64 - size as i64) as f64 / original_size as f64 * 100.0;

        println!(
            "{:<20} | Taille: {:>3} Ko | Gain: {:>6.2}% | Temps: {:?}",
            label,
            size / 1024,
            gain,
            duration
        );

        if gain > 0.0 {
            fs::write(
                format!(
                    "bench_assets/output_{}.webp",
                    label.split_whitespace().next().unwrap().to_lowercase()
                ),
                &processed,
            )?;
        }
    }
    println!("---------------------------------------------");
    println!("Note : Les gains négatifs sur Desktop sont normaux si l'image est redimensionnée");
    println!("       avec une qualité supérieure à sa compression d'origine.");
    println!("=============================================");

    Ok(())
}
