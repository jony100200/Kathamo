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
