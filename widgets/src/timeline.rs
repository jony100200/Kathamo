//! Scrubbable animation/video timeline with tracks and keyframe markers.

use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTrack {
    pub name: String,
    pub keyframes: Vec<u32>, // Frame indices
}

#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    pub current_frame: u32,
    pub total_frames: u32,
    pub fps: u32,
    pub tracks: Vec<TimelineTrack>,
    pub is_playing: bool,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            current_frame: 24,
            total_frames: 240,
            fps: 60,
            tracks: vec![
                TimelineTrack {
                    name: "Sprite Animation".into(),
                    keyframes: vec![0, 15, 30, 45, 60, 90, 120],
                },
                TimelineTrack {
                    name: "Audio Voiceover".into(),
                    keyframes: vec![0, 60, 180],
                },
                TimelineTrack {
                    name: "Camera Pan".into(),
                    keyframes: vec![0, 40, 80, 160, 240],
                },
            ],
            is_playing: false,
        }
    }
}

impl Timeline {
    pub fn scrub_to(&mut self, frame: u32) {
        self.current_frame = frame.min(self.total_frames);
    }

    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Background
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        // Header ruler
        let ruler_height = 28.0;
        let ruler_rect =
            Rect::from_min_size(viewport.min, Vec2::new(viewport.width(), ruler_height));
        buf.push(DrawCommand::Rect {
            rect: ruler_rect,
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        // Frame readout
        buf.push(DrawCommand::Text {
            text: format!(
                "Frame: {}/{} ({} fps)",
                self.current_frame, self.total_frames, self.fps
            ),
            pos: Vec2::new(ruler_rect.min.x + 12.0, ruler_rect.min.y + 7.0),
            size: 12.0,
            color: palette.text,
            strong: true,
        });

        // Tracks area
        let track_start_x = viewport.min.x + 150.0;
        let track_width = (viewport.width() - 160.0).max(10.0);
        let frame_scale = track_width / (self.total_frames as f32).max(1.0);

        let mut y = viewport.min.y + ruler_height + 10.0;
        for track in &self.tracks {
            let track_rect = Rect::from_min_size(
                Vec2::new(viewport.min.x + 10.0, y),
                Vec2::new(viewport.width() - 20.0, 32.0),
            );
            buf.push(DrawCommand::Rect {
                rect: track_rect,
                fill: palette.canvas,
                radius: CornerRadius::same(4.0),
                stroke: Stroke::new(1.0, palette.border),
            });

            // Track name
            buf.push(DrawCommand::Text {
                text: track.name.clone(),
                pos: Vec2::new(track_rect.min.x + 10.0, track_rect.min.y + 9.0),
                size: 12.0,
                color: palette.text_muted,
                strong: false,
            });

            // Keyframe diamonds
            for &kf in &track.keyframes {
                let kf_x = track_start_x + (kf as f32) * frame_scale;
                let kf_rect = Rect::from_min_size(
                    Vec2::new(kf_x - 4.0, track_rect.min.y + 11.0),
                    Vec2::new(8.0, 8.0),
                );
                buf.push(DrawCommand::Rect {
                    rect: kf_rect,
                    fill: palette.warning,
                    radius: CornerRadius::same(2.0),
                    stroke: Stroke::new(1.0, palette.surface),
                });
            }

            y += 40.0;
        }

        // Playhead scrubber line
        let playhead_x = track_start_x + (self.current_frame as f32) * frame_scale;
        buf.push(DrawCommand::Line {
            start: Vec2::new(playhead_x, ruler_rect.min.y),
            end: Vec2::new(playhead_x, viewport.max.y),
            stroke: Stroke::new(2.0, palette.danger),
        });

        // Playhead cap
        let cap_rect = Rect::from_min_size(
            Vec2::new(playhead_x - 6.0, ruler_rect.min.y),
            Vec2::new(12.0, 10.0),
        );
        buf.push(DrawCommand::Rect {
            rect: cap_rect,
            fill: palette.danger,
            radius: CornerRadius::same(2.0),
            stroke: Stroke::NONE,
        });

        buf.push(DrawCommand::ClipPop);
    }
}

/// Renders an audio waveform with vertical amplitude bars and a draggable playhead.
pub fn render_audio_waveform(
    buf: &mut CommandBuffer,
    samples: &[f32],
    playhead_norm: f32,
    bounds: Rect,
    palette: &ThemePalette,
) {
    // Backdrop
    buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: palette.canvas,
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, palette.border),
    });

    if samples.is_empty() || bounds.width() < 10.0 {
        return;
    }

    let bar_width = 3.0;
    let bar_gap = 2.0;
    let step = bar_width + bar_gap;
    let num_bars = ((bounds.width() - 16.0) / step).floor() as usize;

    let center_y = bounds.min.y + bounds.height() * 0.5;
    let max_half_h = (bounds.height() - 12.0) * 0.5;

    let start_x = bounds.min.x + 8.0;
    let playhead_x = bounds.min.x + bounds.width() * playhead_norm.clamp(0.0, 1.0);

    for i in 0..num_bars {
        let sample_idx = (i * samples.len()) / num_bars.max(1);
        let amp = samples
            .get(sample_idx)
            .copied()
            .unwrap_or(0.1)
            .clamp(0.05, 1.0);
        let bar_h = (amp * max_half_h).max(2.0);

        let bar_x = start_x + (i as f32) * step;
        let bar_rect = Rect::from_min_max(
            Vec2::new(bar_x, center_y - bar_h),
            Vec2::new(bar_x + bar_width, center_y + bar_h),
        );

        let is_played = bar_x <= playhead_x;
        let fill = if is_played {
            palette.accent
        } else {
            palette.text_muted.with_alpha(100)
        };

        buf.push(DrawCommand::Rect {
            rect: bar_rect,
            fill,
            radius: CornerRadius::same(1.5),
            stroke: Stroke::NONE,
        });
    }

    // Playhead line
    buf.push(DrawCommand::Line {
        start: Vec2::new(playhead_x, bounds.min.y + 2.0),
        end: Vec2::new(playhead_x, bounds.max.y - 2.0),
        stroke: Stroke::new(2.0, palette.danger),
    });

    // Playhead glowing handle
    buf.push(DrawCommand::Rect {
        rect: Rect::from_min_size(
            Vec2::new(playhead_x - 4.0, bounds.min.y + 2.0),
            Vec2::new(8.0, 8.0),
        ),
        fill: palette.danger,
        radius: CornerRadius::same(2.0),
        stroke: Stroke::NONE,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_and_waveform_render_clean_commands() {
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let tl = Timeline::default();
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 200.0));
        tl.render(&mut buf, viewport, &palette);
        assert!(!buf.is_empty());

        let samples = vec![0.1, 0.4, 0.8, 0.9, 0.5, 0.3, 0.7, 0.2];
        let wave_bounds = Rect::from_min_size(Vec2::new(0.0, 210.0), Vec2::new(400.0, 60.0));
        render_audio_waveform(&mut buf, &samples, 0.5, wave_bounds, &palette);
        assert!(buf.len() > 10);
    }
}
