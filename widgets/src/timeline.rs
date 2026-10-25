//! Scrubbable animation/video timeline with tracks and keyframe markers.

use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTrack {
    pub name: String,
    pub keyframes: Vec<u32>, // Frame indices
}

/// Transient view state. Frames may be fractional; document edits remain app-owned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineView {
    pub start_frame: f64,
    pub visible_frames: f64,
    pub first_track: usize,
}

impl TimelineView {
    pub fn new(visible_frames: f64) -> Self {
        Self {
            start_frame: 0.0,
            visible_frames,
            first_track: 0,
        }
    }

    fn normalized(mut self) -> Self {
        if !self.visible_frames.is_finite() {
            self.visible_frames = 240.0;
        }
        self.visible_frames = self.visible_frames.clamp(1.0, u32::MAX as f64);
        if !self.start_frame.is_finite() {
            self.start_frame = 0.0;
        }
        self.start_frame = self
            .start_frame
            .clamp(0.0, u32::MAX as f64 - self.visible_frames);
        self
    }

    /// Zoom around a pointer position expressed as a proportion of the visible lane.
    pub fn zoom(&mut self, factor: f64, anchor: f64) {
        if !factor.is_finite() || factor <= 0.0 || !anchor.is_finite() {
            return;
        }
        *self = self.normalized();
        let anchor = anchor.clamp(0.0, 1.0);
        let frame = self.start_frame + anchor * self.visible_frames;
        self.visible_frames = (self.visible_frames * factor).clamp(1.0, u32::MAX as f64);
        self.start_frame = frame - anchor * self.visible_frames;
        *self = self.normalized();
    }

    pub fn pan(&mut self, frames: f64) {
        if frames.is_finite() {
            self.start_frame += frames;
            *self = self.normalized();
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimelineClip<'a> {
    pub track: usize,
    pub start_frame: f64,
    pub end_frame: f64,
    pub label: &'a str,
    pub selected: bool,
    pub muted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipEdge {
    Body,
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineHit {
    Ruler,
    Track(usize),
    Clip { index: usize, edge: ClipEdge },
}

/// One geometry authority for rendering, scrubbing, clip picking and trim handles.
#[derive(Debug, Clone, Copy)]
pub struct TimelineGeometry {
    pub bounds: Rect,
    pub lane: Rect,
    pub view: TimelineView,
}

impl TimelineGeometry {
    pub const RULER_HEIGHT: f32 = 30.0;
    pub const ROW_HEIGHT: f32 = 44.0;

    pub fn new(bounds: Rect, view: TimelineView) -> Self {
        let label_width = 128.0_f32.min(bounds.width() * 0.35);
        Self {
            bounds,
            lane: Rect::from_min_max(
                Vec2::new(bounds.min.x + label_width, bounds.min.y),
                bounds.max,
            ),
            view: view.normalized(),
        }
    }

    pub fn frame_x(&self, frame: f64) -> f32 {
        self.lane.min.x
            + ((frame - self.view.start_frame) / self.view.visible_frames) as f32
                * self.lane.width()
    }

    pub fn frame_at(&self, pos: Vec2) -> Option<f64> {
        if !self.lane.contains(pos) || self.lane.width() <= 0.0 {
            return None;
        }
        Some(
            self.view.start_frame
                + ((pos.x - self.lane.min.x) / self.lane.width()) as f64 * self.view.visible_frames,
        )
    }

    pub fn row(&self, track: usize) -> Option<Rect> {
        let offset = track.checked_sub(self.view.first_track)?;
        let y = self.bounds.min.y + Self::RULER_HEIGHT + offset as f32 * Self::ROW_HEIGHT;
        Rect::from_min_size(
            Vec2::new(self.bounds.min.x, y),
            Vec2::new(self.bounds.width(), Self::ROW_HEIGHT),
        )
        .intersect(self.bounds)
    }

    pub fn clip_rect(&self, clip: &TimelineClip<'_>) -> Option<Rect> {
        if !clip.start_frame.is_finite()
            || !clip.end_frame.is_finite()
            || clip.end_frame <= clip.start_frame
        {
            return None;
        }
        let row = self.row(clip.track)?;
        Rect::from_min_max(
            Vec2::new(self.frame_x(clip.start_frame), row.min.y + 5.0),
            Vec2::new(self.frame_x(clip.end_frame), row.max.y - 5.0),
        )
        .intersect(self.lane)
    }

    pub fn hit(
        &self,
        pos: Vec2,
        clips: &[TimelineClip<'_>],
        track_count: usize,
    ) -> Option<TimelineHit> {
        if !self.bounds.contains(pos) {
            return None;
        }
        if pos.y < self.bounds.min.y + Self::RULER_HEIGHT {
            return self.lane.contains(pos).then_some(TimelineHit::Ruler);
        }
        for (index, clip) in clips.iter().enumerate().rev() {
            if self.clip_rect(clip).is_some_and(|r| r.contains(pos)) {
                let edge = if (pos.x - self.frame_x(clip.start_frame)).abs() <= 6.0 {
                    ClipEdge::Start
                } else if (pos.x - self.frame_x(clip.end_frame)).abs() <= 6.0 {
                    ClipEdge::End
                } else {
                    ClipEdge::Body
                };
                return Some(TimelineHit::Clip { index, edge });
            }
        }
        let track = self.view.first_track.saturating_add(
            ((pos.y - self.bounds.min.y - Self::RULER_HEIGHT) / Self::ROW_HEIGHT) as usize,
        );
        (track < track_count).then_some(TimelineHit::Track(track))
    }
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
    /// Render app-provided clip spans without owning playback, decoding or document state.
    pub fn render_clips(
        &self,
        buf: &mut CommandBuffer,
        viewport: Rect,
        palette: &ThemePalette,
        view: TimelineView,
        clips: &[TimelineClip<'_>],
    ) {
        let geometry = TimelineGeometry::new(viewport, view);
        buf.push(DrawCommand::ClipPush { rect: viewport });
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });
        for (index, track) in self.tracks.iter().enumerate().skip(view.first_track) {
            let Some(row) = geometry.row(index) else {
                break;
            };
            buf.push(DrawCommand::Rect {
                rect: row,
                fill: palette.canvas,
                radius: CornerRadius::ZERO,
                stroke: Stroke::new(1.0, palette.border),
            });
            let labels =
                Rect::from_min_max(row.min, Vec2::new(geometry.lane.min.x - 6.0, row.max.y));
            buf.push(DrawCommand::ClipPush { rect: labels });
            buf.push(DrawCommand::Text {
                text: track.name.clone(),
                pos: row.min + Vec2::new(8.0, 14.0),
                size: 12.0,
                color: palette.text_muted,
                strong: false,
            });
            buf.push(DrawCommand::ClipPop);
        }
        buf.push(DrawCommand::ClipPush {
            rect: geometry.lane,
        });
        for clip in clips {
            let Some(rect) = geometry.clip_rect(clip) else {
                continue;
            };
            buf.push(DrawCommand::Rect {
                rect,
                fill: if clip.muted {
                    palette.elevated
                } else {
                    palette.accent.with_alpha(60)
                },
                radius: CornerRadius::same(4.0),
                stroke: Stroke::new(
                    if clip.selected { 2.0 } else { 1.0 },
                    if clip.selected {
                        palette.accent
                    } else {
                        palette.border
                    },
                ),
            });
            buf.push(DrawCommand::ClipPush { rect });
            buf.push(DrawCommand::Text {
                text: clip.label.into(),
                pos: rect.min + Vec2::new(8.0, 9.0),
                size: 12.0,
                color: palette.text,
                strong: clip.selected,
            });
            buf.push(DrawCommand::ClipPop);
            if clip.selected {
                for frame in [clip.start_frame, clip.end_frame] {
                    let x = geometry.frame_x(frame);
                    if x >= geometry.lane.min.x && x <= geometry.lane.max.x {
                        buf.push(DrawCommand::Line {
                            start: Vec2::new(x, rect.min.y + 4.0),
                            end: Vec2::new(x, rect.max.y - 4.0),
                            stroke: Stroke::new(3.0, palette.accent),
                        });
                    }
                }
            }
        }
        for (index, track) in self.tracks.iter().enumerate().skip(view.first_track) {
            let Some(row) = geometry.row(index) else {
                break;
            };
            for frame in &track.keyframes {
                let x = geometry.frame_x(*frame as f64);
                buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(x - 3.0, row.max.y - 10.0),
                        Vec2::new(6.0, 6.0),
                    ),
                    fill: palette.warning,
                    radius: CornerRadius::same(2.0),
                    stroke: Stroke::NONE,
                });
            }
        }
        let ruler = Rect::from_min_size(
            viewport.min,
            Vec2::new(
                viewport.width(),
                TimelineGeometry::RULER_HEIGHT.min(viewport.height()),
            ),
        );
        buf.push(DrawCommand::Rect {
            rect: ruler,
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });
        let ticks = (geometry.lane.width() / 80.0).clamp(1.0, 32.0) as usize;
        for tick in 0..=ticks {
            let frame = geometry.view.start_frame
                + geometry.view.visible_frames * tick as f64 / ticks as f64;
            let x = geometry.frame_x(frame);
            buf.push(DrawCommand::Text {
                text: format!("{:.2}s", frame / self.fps.max(1) as f64),
                pos: Vec2::new(x + 4.0, ruler.min.y + 8.0),
                size: 11.0,
                color: palette.text_muted,
                strong: false,
            });
        }
        let x = geometry.frame_x(self.current_frame as f64);
        buf.push(DrawCommand::Line {
            start: Vec2::new(x, ruler.min.y),
            end: Vec2::new(x, viewport.max.y),
            stroke: Stroke::new(2.0, palette.danger),
        });
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(x - 5.0, ruler.min.y), Vec2::new(10.0, 8.0)),
            fill: palette.danger,
            radius: CornerRadius::same(2.0),
            stroke: Stroke::NONE,
        });
        buf.push(DrawCommand::ClipPop);
        buf.push(DrawCommand::ClipPop);
    }

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
