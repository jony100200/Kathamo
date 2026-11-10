//! Audio level meter: green up to -12 dB, yellow up to -3 dB, red above, with the unlit part
//! of each zone dimmed and a 2 px peak-hold line. The scale gives the loud end most of the
//! room: -80..-40 dB fills the first 10 %, -40..-20 dB the next 20 %, -20..0 dB the last 70 %.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

/// Zone colours.
const GREEN: Color = Color::rgb(0x3c, 0xc8, 0x50);
const YELLOW: Color = Color::rgb(0xe6, 0xd2, 0x3c);
const RED: Color = Color::rgb(0xe6, 0x32, 0x28);
/// How bright the unlit part of a zone is (0..1).
const UNLIT: f32 = 0.18;

/// Where `db` sits on the meter, 0 (silent) to 1 (0 dBFS).
pub fn db_to_fraction(db: f32) -> f32 {
    if !db.is_finite() || db <= -80.0 {
        return 0.0;
    }
    let db = db.min(0.0);
    if db < -40.0 {
        (db + 80.0) / 40.0 * 0.1
    } else if db < -20.0 {
        0.1 + (db + 40.0) / 20.0 * 0.2
    } else {
        0.3 + (db + 20.0) / 20.0 * 0.7
    }
}

/// A linear amplitude (0..1) in dBFS.
pub fn amplitude_to_db(amplitude: f32) -> f32 {
    if amplitude <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * amplitude.log10()
    }
}

fn dim(c: Color) -> Color {
    let m = |x: u8| (x as f32 * UNLIT).round() as u8;
    Color::rgba(m(c.r), m(c.g), m(c.b), c.a)
}

/// Draws one meter in `rect`, filling left to right when it is wider than tall, else bottom
/// to top. `level_db` is the current level, `peak_db` the held peak, if any.
pub fn draw_meter(cmd: &mut CommandBuffer, rect: Rect, level_db: f32, peak_db: Option<f32>) {
    let horizontal = rect.width() >= rect.height();
    let length = if horizontal {
        rect.width()
    } else {
        rect.height()
    };
    // The span of the bar between two fractions of its length.
    let span = |from: f32, to: f32| -> Rect {
        if horizontal {
            Rect::from_min_max(
                Vec2::new(rect.min.x + from * length, rect.min.y),
                Vec2::new(rect.min.x + to * length, rect.max.y),
            )
        } else {
            Rect::from_min_max(
                Vec2::new(rect.min.x, rect.max.y - to * length),
                Vec2::new(rect.max.x, rect.max.y - from * length),
            )
        }
    };
    let lit = db_to_fraction(level_db);
    let zones = [
        (0.0, db_to_fraction(-12.0), GREEN),
        (db_to_fraction(-12.0), db_to_fraction(-3.0), YELLOW),
        (db_to_fraction(-3.0), 1.0, RED),
    ];
    for (from, to, color) in zones {
        let fill = |cmd: &mut CommandBuffer, a: f32, b: f32, c: Color| {
            if b > a {
                cmd.push(DrawCommand::Rect {
                    rect: span(a, b),
                    fill: c,
                    radius: CornerRadius::ZERO,
                    stroke: Stroke::NONE,
                });
            }
        };
        let split = lit.clamp(from, to);
        fill(cmd, from, split, color);
        fill(cmd, split, to, dim(color));
    }
    if let Some(peak) = peak_db {
        let at = db_to_fraction(peak);
        if at > 0.0 {
            let color = zones
                .iter()
                .find(|(from, to, _)| at >= *from && at <= *to)
                .map_or(GREEN, |z| z.2);
            let half = 1.0 / length;
            cmd.push(DrawCommand::Rect {
                rect: span((at - half).max(0.0), (at + half).min(1.0)),
                fill: color,
                radius: CornerRadius::ZERO,
                stroke: Stroke::NONE,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_gives_the_loud_end_most_of_the_room() {
        assert_eq!(db_to_fraction(-90.0), 0.0);
        assert!((db_to_fraction(-40.0) - 0.1).abs() < 1e-6);
        assert!((db_to_fraction(-20.0) - 0.3).abs() < 1e-6);
        assert!((db_to_fraction(0.0) - 1.0).abs() < 1e-6);
        assert_eq!(db_to_fraction(6.0), 1.0, "clipped at full scale");
        assert_eq!(db_to_fraction(f32::NEG_INFINITY), 0.0);
        assert!((amplitude_to_db(0.5) + 6.0206).abs() < 0.01);
    }

    #[test]
    fn a_loud_level_lights_into_the_yellow_and_the_peak_sits_above_it() {
        let mut cmd = CommandBuffer::default();
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 8.0));
        draw_meter(&mut cmd, rect, -6.0, Some(-2.0));
        let fills: Vec<(Rect, Color)> = cmd
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Rect { rect, fill, .. } => Some((*rect, *fill)),
                _ => None,
            })
            .collect();
        assert!(fills.iter().any(|(_, c)| *c == GREEN));
        assert!(
            fills.iter().any(|(_, c)| *c == YELLOW),
            "-6 dB is in the yellow"
        );
        assert!(fills.iter().any(|(_, c)| *c == dim(RED)), "red stays unlit");
        let (peak, color) = fills.last().unwrap();
        assert_eq!(*color, RED, "the -2 dB peak hold is drawn in the red zone");
        assert!(peak.width() <= 2.01);
    }
}
