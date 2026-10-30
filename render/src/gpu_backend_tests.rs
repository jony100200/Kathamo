//! GPU backend tests: real headless rendering, texture sampling and device-loss recovery.

use super::*;

fn headless() -> Option<GpuBackend> {
    match GpuBackend::new_headless(320, 200) {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            None
        }
    }
}

#[test]
fn gpu_backend_samples_application_rgba_texture() {
    let Some(mut gpu) = headless() else { return };
    let texels = [255u8, 0, 0, 255, 0, 255, 0, 255];
    gpu.upload_rgba8_image(77, 2, 1, &texels).unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(DrawCommand::Image {
        rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(320.0, 200.0)),
        texture_id: 77,
        uv_rect: Rect::from_min_max(Vec2::ZERO, Vec2::new(1.0, 1.0)),
        tint: Color::WHITE,
    });
    gpu.begin_frame(320, 200, Color::BLACK);
    gpu.submit(&commands);
    gpu.end_frame().unwrap();
    let pixels = gpu.read_pixels().unwrap();
    assert_eq!(pixels[100 * 320 + 60], 0xFFFF_0000);
    assert_eq!(pixels[100 * 320 + 260], 0xFF00_FF00);
    gpu.upload_rgba8_image(77, 2, 1, &texels).unwrap();
    assert_eq!(gpu.images.len(), 1, "video frames should update in-place");
}

#[test]
fn gpu_backend_renders_real_pixels_and_recovers_from_real_device_loss() {
    let Some(mut gpu) = headless() else { return };
    let mut buf = CommandBuffer::new();
    buf.push(DrawCommand::Rect {
        rect: Rect::from_min_size(Vec2::new(20.0, 20.0), Vec2::new(100.0, 60.0)),
        fill: Color::rgb(255, 0, 0),
        radius: CornerRadius::same(8.0),
        stroke: Stroke::NONE,
    });
    buf.push(DrawCommand::Text {
        text: "GPU কল্পনা".into(),
        pos: Vec2::new(20.0, 120.0),
        size: 18.0,
        color: Color::WHITE,
        strong: false,
    });

    gpu.begin_frame(320, 200, Color::BLACK);
    let stats = gpu.submit(&buf);
    assert!(stats.vertices > 8);
    assert!(gpu.end_frame().is_ok());
    let pixels = gpu.read_pixels().unwrap();
    assert_eq!(
        pixels[50 * 320 + 70],
        0xFFFF_0000,
        "rect centre must be red"
    );
    assert_eq!(
        pixels[5 * 320 + 5],
        0xFF00_0000,
        "background must be clear colour"
    );
    let text_lit = pixels[110 * 320..150 * 320]
        .iter()
        .filter(|&&p| p & 0xFF > 0)
        .count();
    assert!(text_lit > 50, "text must be drawn on the GPU");

    gpu.simulate_device_loss();
    assert!(
        gpu.end_frame().is_err(),
        "destroyed device must fail the frame"
    );
    gpu.recover().expect("device must be recreated");
    gpu.begin_frame(320, 200, Color::BLACK);
    gpu.submit(&buf);
    assert!(gpu.end_frame().is_ok());
    assert_eq!(gpu.read_pixels().unwrap()[50 * 320 + 70], 0xFFFF_0000);
}
