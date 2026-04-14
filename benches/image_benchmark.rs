use api_s3::{
    models::device::{DeviceConfig, DeviceTarget},
    services::image_processor,
};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tokio::runtime::Runtime;

fn benchmark_full_process(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let img_path = "bench_assets/benchmark.jpg";
    let image_data = std::fs::read(img_path).expect("Benchmark image not found");

    let config = DeviceConfig {
        target: DeviceTarget::Phone,
        width: 50,
        height: 50,
        scale: None,
    };

    let mut group = c.benchmark_group("full_process");
    group.throughput(Throughput::Bytes(image_data.len() as u64));

    group.bench_function("Process (Phone 720x1600 - Resize)", |b| {
        b.to_async(&rt).iter(|| {
            image_processor::process_image(black_box(image_data.clone()), black_box(config.clone()))
        })
    });

    let config_bypass = DeviceConfig {
        target: DeviceTarget::Desktop,
        width: 2000,
        height: 2000,
        scale: None,
    };

    group.bench_function("Process (Bypass - Instant)", |b| {
        b.to_async(&rt).iter(|| {
            image_processor::process_image(
                black_box(image_data.clone()),
                black_box(config_bypass.clone()),
            )
        })
    });

    group.finish();
}

fn benchmark_granular(c: &mut Criterion) {
    let img_path = "bench_assets/benchmark.jpg";
    let image_data = std::fs::read(img_path).expect("Benchmark image not found");

    let mut group = c.benchmark_group("granular_steps");

    // Configuration du décodage
    group.throughput(Throughput::Bytes(image_data.len() as u64));
    group.bench_function("Etape 1: Decodage", |b| {
        b.iter(|| image_processor::decode_image(black_box(&image_data)))
    });

    // Configuration du redimensionnement
    let src_image = image_processor::decode_image(&image_data).unwrap();
    let (w, h) = (src_image.width(), src_image.height());
    let config = DeviceConfig {
        target: DeviceTarget::Phone,
        width: 1200,
        height: 450,
        scale: None,
    };
    let (target_w, target_h) = image_processor::compute_target_dimensions(w, h, &config);

    // le débit pour le redimensionnement pourrait être les pixels d'entrée * octets par pixel
    // w * h * 4 (RGBA)
    group.throughput(Throughput::Bytes((w * h * 4) as u64));
    group.bench_function("Etape 2: Redimensionnement (SIMD)", |b| {
        b.iter(|| image_processor::resize_image(black_box(&src_image), w, h, target_w, target_h))
    });

    // Configuration de l'encodage
    let (resized_buffer, pixel_type) =
        image_processor::resize_image(&src_image, w, h, target_w, target_h).unwrap();

    let quality = config.target.default_quality();
    // le débit pour l'encodage est généralement les octets bruts en entrée -> octets encodés en sortie,
    // mais typiquement nous mesurons la vitesse de traitement de l'entrée
    group.throughput(Throughput::Bytes((target_w * target_h * 4) as u64));
    group.bench_function("Etape 3: Encodage (WebP)", |b| {
        b.iter(|| {
            image_processor::encode_image(
                black_box(&resized_buffer),
                target_w,
                target_h,
                pixel_type,
                quality,
            )
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_full_process, benchmark_granular);
criterion_main!(benches);
