use std::time::{Duration, Instant};

use crate::core::context::{ContextId, MiniContent, Priority};

pub const SPLIT_DURATION: Duration = Duration::from_millis(280);
pub const MERGE_DURATION: Duration = Duration::from_millis(240);
pub const REPLACE_DURATION: Duration = Duration::from_millis(200);
pub const SWAP_DURATION: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MultitaskTaskId {
    Music,
    Plugin(ContextId),
}

#[derive(Debug, Clone)]
pub struct MultitaskTask {
    pub id: MultitaskTaskId,
    pub content: MiniContent,
    pub priority: Priority,
    pub created_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultitaskTransitionKind {
    Stable,
    Split,
    Merge,
    Promote,
    Replace,
    Swap,
}

#[derive(Debug, Clone, Copy)]
pub struct MultitaskFrame {
    pub kind: MultitaskTransitionKind,
    pub main_width_scale: f32,
    pub main_radius_scale: f32,
    pub main_offset_units: f32,
    pub main_alpha: f32,
    pub secondary_scale: f32,
    pub secondary_alpha: f32,
    pub secondary_content_alpha: f32,
    pub secondary_slide_units: f32,
    pub outgoing_secondary_scale: f32,
    pub outgoing_secondary_alpha: f32,
    pub gap_progress: f32,
    pub bridge_progress: f32,
    pub mask_alpha: f32,
    pub breath_scale: f32,
    pub promote_progress: f32,
}

impl Default for MultitaskFrame {
    fn default() -> Self {
        Self {
            kind: MultitaskTransitionKind::Stable,
            main_width_scale: 1.0,
            main_radius_scale: 1.0,
            main_offset_units: 0.0,
            main_alpha: 1.0,
            secondary_scale: 0.0,
            secondary_alpha: 0.0,
            secondary_content_alpha: 0.0,
            secondary_slide_units: 0.0,
            outgoing_secondary_scale: 0.0,
            outgoing_secondary_alpha: 0.0,
            gap_progress: 0.0,
            bridge_progress: 0.0,
            mask_alpha: 0.0,
            breath_scale: 1.0,
            promote_progress: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
enum MultitaskTransition {
    #[default]
    Stable,
    Split {
        started_at: Instant,
    },
    Merge {
        started_at: Instant,
        outgoing: MultitaskTask,
    },
    Promote {
        started_at: Instant,
    },
    Replace {
        started_at: Instant,
        outgoing: MultitaskTask,
    },
    Swap {
        started_at: Instant,
        old_primary: MultitaskTask,
        old_secondary: MultitaskTask,
    },
}

impl MultitaskTransition {
    fn duration(&self) -> Duration {
        match self {
            Self::Stable => Duration::ZERO,
            Self::Split { .. } => SPLIT_DURATION,
            Self::Merge { .. } | Self::Promote { .. } => MERGE_DURATION,
            Self::Replace { .. } => REPLACE_DURATION,
            Self::Swap { .. } => SWAP_DURATION,
        }
    }

    fn started_at(&self) -> Option<Instant> {
        match self {
            Self::Stable => None,
            Self::Split { started_at }
            | Self::Merge { started_at, .. }
            | Self::Promote { started_at }
            | Self::Replace { started_at, .. }
            | Self::Swap { started_at, .. } => Some(*started_at),
        }
    }
}

#[derive(Debug, Default)]
pub struct MultitaskController {
    primary: Option<MultitaskTask>,
    secondary: Option<MultitaskTask>,
    transition: MultitaskTransition,
}

impl MultitaskController {
    pub fn primary(&self) -> Option<&MultitaskTask> {
        self.primary.as_ref()
    }

    pub fn secondary(&self) -> Option<&MultitaskTask> {
        self.secondary.as_ref()
    }

    pub fn transition_kind(&self) -> MultitaskTransitionKind {
        match self.transition {
            MultitaskTransition::Stable => MultitaskTransitionKind::Stable,
            MultitaskTransition::Split { .. } => MultitaskTransitionKind::Split,
            MultitaskTransition::Merge { .. } => MultitaskTransitionKind::Merge,
            MultitaskTransition::Promote { .. } => MultitaskTransitionKind::Promote,
            MultitaskTransition::Replace { .. } => MultitaskTransitionKind::Replace,
            MultitaskTransition::Swap { .. } => MultitaskTransitionKind::Swap,
        }
    }

    pub fn is_animating(&self) -> bool {
        !matches!(self.transition, MultitaskTransition::Stable)
    }

    pub fn reconcile(&mut self, candidates: &[MultitaskTask], now: Instant) {
        self.finish_transition(now);
        if self.is_animating() {
            return;
        }

        let contains = |task: &MultitaskTask| candidates.iter().any(|item| item.id == task.id);
        match (&self.primary, &self.secondary) {
            (None, _) => {
                self.primary = candidates.first().cloned();
                self.secondary = None;
            }
            (Some(primary), Some(secondary)) if !contains(primary) => {
                if contains(secondary) {
                    self.primary = Some(secondary.clone());
                    self.secondary = None;
                    self.transition = MultitaskTransition::Promote { started_at: now };
                } else {
                    self.primary = candidates.first().cloned();
                    self.secondary = None;
                }
            }
            (Some(primary), Some(secondary)) if !contains(secondary) => {
                let replacement = best_candidate(candidates, &[&primary.id]);
                if let Some(replacement) = replacement {
                    let outgoing = secondary.clone();
                    self.secondary = Some(replacement);
                    self.transition = MultitaskTransition::Replace {
                        started_at: now,
                        outgoing,
                    };
                } else {
                    let outgoing = secondary.clone();
                    self.secondary = None;
                    self.transition = MultitaskTransition::Merge {
                        started_at: now,
                        outgoing,
                    };
                }
            }
            (Some(primary), Some(secondary)) => {
                let incoming = best_candidate(candidates, &[&primary.id, &secondary.id]);
                if let Some(incoming) = incoming
                    && outranks(&incoming, secondary)
                {
                    let outgoing = secondary.clone();
                    self.secondary = Some(incoming);
                    self.transition = MultitaskTransition::Replace {
                        started_at: now,
                        outgoing,
                    };
                }
            }
            (Some(primary), None) if !contains(primary) => {
                self.primary = candidates.first().cloned();
            }
            (Some(primary), None) => {
                if let Some(secondary) = best_candidate(candidates, &[&primary.id]) {
                    self.secondary = Some(secondary);
                    self.transition = MultitaskTransition::Split { started_at: now };
                }
            }
        }
    }

    pub fn swap(&mut self, now: Instant) -> bool {
        self.finish_transition(now);
        if self.is_animating() {
            return false;
        }
        let (Some(primary), Some(secondary)) = (&self.primary, &self.secondary) else {
            return false;
        };
        self.transition = MultitaskTransition::Swap {
            started_at: now,
            old_primary: primary.clone(),
            old_secondary: secondary.clone(),
        };
        true
    }

    pub fn display_tasks(&self, now: Instant) -> (Option<MultitaskTask>, Option<MultitaskTask>) {
        if let MultitaskTransition::Swap {
            started_at,
            old_primary,
            old_secondary,
        } = &self.transition
        {
            if elapsed_ms(now, *started_at) >= 100.0 {
                return (Some(old_secondary.clone()), Some(old_primary.clone()));
            }
            return (Some(old_primary.clone()), Some(old_secondary.clone()));
        }
        (self.primary.clone(), self.secondary.clone())
    }

    pub fn outgoing_secondary(&self) -> Option<&MultitaskTask> {
        match &self.transition {
            MultitaskTransition::Merge { outgoing, .. }
            | MultitaskTransition::Replace { outgoing, .. } => Some(outgoing),
            _ => None,
        }
    }

    pub fn frame(&self, now: Instant) -> MultitaskFrame {
        match &self.transition {
            MultitaskTransition::Stable => stable_frame(self.secondary.is_some()),
            MultitaskTransition::Split { started_at } => split_frame(elapsed_ms(now, *started_at)),
            MultitaskTransition::Merge { started_at, .. } => {
                merge_frame(elapsed_ms(now, *started_at), false)
            }
            MultitaskTransition::Promote { started_at } => {
                merge_frame(elapsed_ms(now, *started_at), true)
            }
            MultitaskTransition::Replace { started_at, .. } => {
                replace_frame(elapsed_ms(now, *started_at))
            }
            MultitaskTransition::Swap { started_at, .. } => {
                swap_frame(elapsed_ms(now, *started_at))
            }
        }
    }

    fn finish_transition(&mut self, now: Instant) {
        let Some(started_at) = self.transition.started_at() else {
            return;
        };
        if now.saturating_duration_since(started_at) < self.transition.duration() {
            return;
        }
        if let MultitaskTransition::Swap {
            old_primary,
            old_secondary,
            ..
        } = &self.transition
        {
            self.primary = Some(old_secondary.clone());
            self.secondary = Some(old_primary.clone());
        }
        self.transition = MultitaskTransition::Stable;
    }
}

fn best_candidate(
    candidates: &[MultitaskTask],
    excluded: &[&MultitaskTaskId],
) -> Option<MultitaskTask> {
    candidates
        .iter()
        .find(|candidate| !excluded.contains(&&candidate.id))
        .cloned()
}

fn outranks(incoming: &MultitaskTask, current: &MultitaskTask) -> bool {
    incoming.priority > current.priority
        || (incoming.priority == current.priority && incoming.created_at > current.created_at)
}

fn elapsed_ms(now: Instant, started_at: Instant) -> f32 {
    now.saturating_duration_since(started_at).as_secs_f32() * 1000.0
}

fn unit(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn smooth(value: f32) -> f32 {
    let value = unit(value);
    value * value * (3.0 - 2.0 * value)
}

fn stable_frame(dual: bool) -> MultitaskFrame {
    MultitaskFrame {
        secondary_scale: if dual { 1.0 } else { 0.0 },
        secondary_alpha: if dual { 1.0 } else { 0.0 },
        secondary_content_alpha: if dual { 1.0 } else { 0.0 },
        gap_progress: if dual { 1.0 } else { 0.0 },
        ..Default::default()
    }
}

fn split_frame(ms: f32) -> MultitaskFrame {
    let mut frame = MultitaskFrame {
        kind: MultitaskTransitionKind::Split,
        bridge_progress: 1.0,
        ..Default::default()
    };
    if ms < 50.0 {
        let progress = smooth(ms / 50.0);
        frame.main_width_scale = 1.0 + 0.06 * progress;
        frame.main_radius_scale = 1.0 - 0.08 * progress;
        frame.mask_alpha = 0.10 * progress;
    } else if ms < 126.0 {
        let progress = smooth((ms - 50.0) / 76.0);
        frame.main_width_scale = 1.06 - 0.06 * progress;
        frame.main_radius_scale = 0.92 + 0.04 * progress;
        frame.main_offset_units = -4.0 * progress;
        frame.gap_progress = progress;
        frame.bridge_progress = 1.0 - progress;
        frame.mask_alpha = 0.10 * (1.0 - progress * 0.65);
        frame.secondary_scale = 1.0 - 0.28 * progress;
        frame.secondary_alpha = progress;
    } else if ms < 218.0 {
        let progress = smooth((ms - 126.0) / 92.0);
        frame.main_radius_scale = 0.96 + 0.04 * progress;
        frame.main_offset_units = -4.0 * (1.0 - progress);
        frame.secondary_scale = 0.72 + 0.28 * progress;
        frame.secondary_alpha = 1.0;
        frame.secondary_content_alpha = progress;
        frame.gap_progress = 1.0;
        frame.mask_alpha = 0.035 * (1.0 - progress);
    } else {
        let progress = unit((ms - 218.0) / 62.0);
        let rebound = (std::f32::consts::PI * progress).sin() * (1.0 - progress);
        frame.secondary_scale = 1.0 + 0.05 * rebound;
        frame.secondary_alpha = 1.0;
        frame.secondary_content_alpha = 1.0;
        frame.secondary_slide_units = -3.0 * rebound;
        frame.gap_progress = 1.0;
    }
    frame
}

fn merge_frame(ms: f32, promote: bool) -> MultitaskFrame {
    let mut frame = MultitaskFrame {
        kind: if promote {
            MultitaskTransitionKind::Promote
        } else {
            MultitaskTransitionKind::Merge
        },
        secondary_scale: 1.0,
        secondary_alpha: 1.0,
        secondary_content_alpha: 1.0,
        outgoing_secondary_scale: 1.0,
        outgoing_secondary_alpha: 1.0,
        gap_progress: 1.0,
        promote_progress: if promote { 0.0 } else { 1.0 },
        ..Default::default()
    };
    if ms < 70.0 {
        let progress = smooth(ms / 70.0);
        frame.outgoing_secondary_scale = 1.0 - 0.65 * progress;
        frame.outgoing_secondary_alpha = 1.0 - progress;
        frame.secondary_scale = frame.outgoing_secondary_scale;
        frame.secondary_alpha = frame.outgoing_secondary_alpha;
        frame.secondary_content_alpha = frame.secondary_alpha;
        frame.main_alpha = if promote { 0.0 } else { 1.0 };
    } else if ms < 156.0 {
        let progress = smooth((ms - 70.0) / 86.0);
        frame.secondary_scale = 0.0;
        frame.secondary_alpha = 0.0;
        frame.outgoing_secondary_scale = 0.35 * (1.0 - progress);
        frame.outgoing_secondary_alpha = 0.0;
        frame.gap_progress = 1.0 - progress;
        frame.bridge_progress = progress * (1.0 - progress);
        frame.mask_alpha = 0.10 * (1.0 - progress);
        frame.promote_progress = if promote { progress } else { 1.0 };
        frame.main_alpha = if promote { progress } else { 1.0 };
    } else {
        let progress = unit((ms - 156.0) / 84.0);
        frame.secondary_scale = 0.0;
        frame.secondary_alpha = 0.0;
        frame.gap_progress = 0.0;
        frame.promote_progress = 1.0;
        frame.main_alpha = 1.0;
        frame.breath_scale = 1.0 + 0.025 * (std::f32::consts::PI * progress).sin();
    }
    frame
}

fn replace_frame(ms: f32) -> MultitaskFrame {
    let outgoing_progress = smooth(ms / 80.0);
    let incoming_progress = smooth((ms - 60.0) / 140.0);
    let mut outgoing_alpha = 1.0 - outgoing_progress;
    let mut incoming_alpha = incoming_progress;
    let sum = outgoing_alpha + incoming_alpha;
    if sum > 1.0 {
        outgoing_alpha /= sum;
        incoming_alpha /= sum;
    }
    MultitaskFrame {
        kind: MultitaskTransitionKind::Replace,
        secondary_scale: 0.72 + 0.28 * incoming_progress,
        secondary_alpha: incoming_alpha,
        secondary_content_alpha: incoming_alpha,
        secondary_slide_units: 10.0 * (1.0 - incoming_progress),
        outgoing_secondary_scale: 1.0 - 0.40 * outgoing_progress,
        outgoing_secondary_alpha: outgoing_alpha,
        gap_progress: 1.0,
        ..Default::default()
    }
}

fn swap_frame(ms: f32) -> MultitaskFrame {
    let first_half = ms < 100.0;
    let progress = if first_half {
        smooth(ms / 100.0)
    } else {
        smooth((ms - 100.0) / 100.0)
    };
    let alpha = if first_half { 1.0 - progress } else { progress };
    let scale = if first_half {
        1.0 - 0.12 * progress
    } else {
        0.88 + 0.12 * progress
    };
    MultitaskFrame {
        kind: MultitaskTransitionKind::Swap,
        main_alpha: alpha,
        secondary_scale: scale,
        secondary_alpha: alpha,
        secondary_content_alpha: alpha,
        gap_progress: 1.0,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::PluginContext;

    fn task(source: &str, priority: Priority, created_at: Instant) -> MultitaskTask {
        let context = PluginContext {
            id: ContextId::new(source),
            priority,
            title: source.to_string(),
            body: String::new(),
            icon: Vec::new(),
            duration_sec: 10,
            mini_render: true,
            mini_text: source.to_string(),
            created_at,
            expanded_started_at: None,
            collapsed_at: None,
            mini_timeout_start: None,
        };
        MultitaskTask {
            id: MultitaskTaskId::Plugin(context.id.clone()),
            content: MiniContent::Plugin(Box::new(context)),
            priority,
            created_at,
        }
    }

    #[test]
    fn split_preserves_existing_primary() {
        let now = Instant::now();
        let first = task("first", Priority::Low, now);
        let second = task("second", Priority::High, now + Duration::from_millis(1));
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), now);
        controller.reconcile(&[second.clone(), first.clone()], now);
        assert_eq!(controller.primary().map(|item| &item.id), Some(&first.id));
        assert_eq!(
            controller.secondary().map(|item| &item.id),
            Some(&second.id)
        );
        assert_eq!(controller.transition_kind(), MultitaskTransitionKind::Split);
    }

    #[test]
    fn higher_or_newer_third_task_replaces_secondary() {
        let now = Instant::now();
        let first = task("first", Priority::Low, now);
        let second = task("second", Priority::Medium, now + Duration::from_millis(1));
        let third = task("third", Priority::High, now + Duration::from_millis(2));
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), now);
        controller.reconcile(&[second.clone(), first.clone()], now);
        controller.reconcile(&[third.clone(), second, first], now + SPLIT_DURATION);
        assert_eq!(controller.secondary().map(|item| &item.id), Some(&third.id));
        assert_eq!(
            controller.transition_kind(),
            MultitaskTransitionKind::Replace
        );
    }

    #[test]
    fn lower_priority_third_task_does_not_replace_secondary() {
        let now = Instant::now();
        let first = task("first", Priority::Low, now);
        let second = task("second", Priority::High, now + Duration::from_millis(1));
        let third = task("third", Priority::Medium, now + Duration::from_millis(2));
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), now);
        controller.reconcile(&[second.clone(), first.clone()], now);
        controller.reconcile(&[second.clone(), third, first], now + SPLIT_DURATION);
        assert_eq!(
            controller.secondary().map(|item| &item.id),
            Some(&second.id)
        );
        assert_eq!(
            controller.transition_kind(),
            MultitaskTransitionKind::Stable
        );
    }

    #[test]
    fn newer_equal_priority_task_replaces_secondary() {
        let now = Instant::now();
        let first = task("first", Priority::Low, now);
        let second = task("second", Priority::High, now + Duration::from_millis(1));
        let third = task("third", Priority::High, now + Duration::from_millis(2));
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), now);
        controller.reconcile(&[second.clone(), first.clone()], now);
        controller.reconcile(&[third.clone(), second, first], now + SPLIT_DURATION);
        assert_eq!(controller.secondary().map(|item| &item.id), Some(&third.id));
        assert_eq!(
            controller.transition_kind(),
            MultitaskTransitionKind::Replace
        );
    }

    #[test]
    fn ending_secondary_merges_to_single_task() {
        let now = Instant::now();
        let first = task("first", Priority::Low, now);
        let second = task("second", Priority::High, now + Duration::from_millis(1));
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), now);
        controller.reconcile(&[second, first.clone()], now);
        controller.reconcile(std::slice::from_ref(&first), now + SPLIT_DURATION);
        assert_eq!(controller.transition_kind(), MultitaskTransitionKind::Merge);
        assert!(controller.secondary().is_none());
        assert!(controller.outgoing_secondary().is_some());
    }

    #[test]
    fn ending_primary_promotes_secondary() {
        let now = Instant::now();
        let first = task("first", Priority::Low, now);
        let second = task("second", Priority::High, now + Duration::from_millis(1));
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), now);
        controller.reconcile(&[second.clone(), first], now);
        controller.reconcile(std::slice::from_ref(&second), now + SPLIT_DURATION);
        assert_eq!(
            controller.transition_kind(),
            MultitaskTransitionKind::Promote
        );
        assert_eq!(controller.primary().map(|item| &item.id), Some(&second.id));
        assert!(controller.secondary().is_none());
    }

    #[test]
    fn task_change_during_split_is_reconciled_after_split() {
        let now = Instant::now();
        let first = task("first", Priority::Low, now);
        let second = task("second", Priority::Medium, now + Duration::from_millis(1));
        let third = task("third", Priority::High, now + Duration::from_millis(2));
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), now);
        controller.reconcile(&[second.clone(), first.clone()], now);
        controller.reconcile(
            &[third.clone(), second.clone(), first.clone()],
            now + Duration::from_millis(100),
        );
        assert_eq!(
            controller.secondary().map(|item| &item.id),
            Some(&second.id)
        );
        controller.reconcile(&[third.clone(), second, first], now + SPLIT_DURATION);
        assert_eq!(controller.secondary().map(|item| &item.id), Some(&third.id));
        assert_eq!(
            controller.transition_kind(),
            MultitaskTransitionKind::Replace
        );
    }

    #[test]
    fn replacement_frame_never_moves_main_or_exceeds_total_alpha() {
        for ms in [0, 60, 80, 100, 200] {
            let frame = replace_frame(ms as f32);
            assert_eq!(frame.main_width_scale, 1.0);
            assert_eq!(frame.main_radius_scale, 1.0);
            assert_eq!(frame.main_offset_units, 0.0);
            assert!(frame.secondary_alpha + frame.outgoing_secondary_alpha <= 1.001);
        }
    }

    #[test]
    fn split_and_merge_frames_hit_required_keyframes() {
        let split_start = Instant::now();
        let first = task("first", Priority::Low, split_start);
        let second = task("second", Priority::High, split_start);
        let mut controller = MultitaskController::default();
        controller.reconcile(std::slice::from_ref(&first), split_start);
        controller.reconcile(&[second.clone(), first.clone()], split_start);
        let pre = controller.frame(split_start + Duration::from_millis(50));
        let shaped = controller.frame(split_start + Duration::from_millis(218));
        assert!((pre.main_width_scale - 1.06).abs() < 0.001);
        assert_eq!(shaped.gap_progress, 1.0);

        controller.reconcile(std::slice::from_ref(&first), split_start + SPLIT_DURATION);
        let merged = controller.frame(split_start + SPLIT_DURATION + MERGE_DURATION);
        assert!(merged.secondary_alpha <= 0.001);
        assert!((merged.breath_scale - 1.0).abs() < 0.001);
    }
}
