mod notification;
mod volume;

use skia_safe::{Canvas, Rect};

use self::notification::{NotificationIndicator, NotificationMonitor};
use self::volume::{VolumeIndicator, VolumeMonitor};

#[derive(Clone, Copy)]
pub struct CompactSize {
    pub width: f32,
    pub height: f32,
}

pub struct CompactOverlay {
    volume_monitor: VolumeMonitor,
    volume_indicator: VolumeIndicator,
    notification_monitor: NotificationMonitor,
    notification_indicator: NotificationIndicator,
}

enum ActiveCompactOverlay<'a> {
    Notification(&'a NotificationIndicator),
    Volume(&'a VolumeIndicator),
}

impl ActiveCompactOverlay<'_> {
    fn target_size(&self, base_width: f32, base_height: f32, scale: f32) -> CompactSize {
        match self {
            Self::Notification(indicator) => indicator.target_size(base_width, base_height, scale),
            Self::Volume(indicator) => indicator.target_size(base_width, base_height, scale),
        }
    }

    fn draw(&self, canvas: &Canvas, rect: Rect, scale: f32, alpha: f32) {
        match self {
            Self::Notification(indicator) => indicator.draw(canvas, rect, scale, alpha),
            Self::Volume(indicator) => indicator.draw(canvas, rect, scale, alpha),
        }
    }
}

impl Default for CompactOverlay {
    fn default() -> Self {
        Self {
            volume_monitor: VolumeMonitor::new(),
            volume_indicator: VolumeIndicator::default(),
            notification_monitor: NotificationMonitor::default(),
            notification_indicator: NotificationIndicator::default(),
        }
    }
}

impl CompactOverlay {
    pub fn update(&mut self, can_present: bool, notification_display: bool) {
        self.volume_indicator
            .update(self.volume_monitor.snapshot(), can_present);
        let notification = self.notification_monitor.update(notification_display);
        if notification_display {
            self.notification_indicator
                .update(notification, can_present);
        } else {
            self.notification_indicator.clear();
        }
    }

    pub fn is_visible(&self) -> bool {
        self.active().is_some()
    }

    pub fn is_notification_visible(&self) -> bool {
        self.notification_indicator.is_visible()
    }

    pub fn target_size(
        &self,
        base_width: f32,
        base_height: f32,
        scale: f32,
    ) -> Option<CompactSize> {
        self.active()
            .map(|overlay| overlay.target_size(base_width, base_height, scale))
    }

    pub fn draw(&self, canvas: &Canvas, rect: Rect, scale: f32, alpha: f32) {
        if let Some(overlay) = self.active() {
            overlay.draw(canvas, rect, scale, alpha);
        }
    }

    fn active(&self) -> Option<ActiveCompactOverlay<'_>> {
        if self.notification_indicator.is_visible() {
            Some(ActiveCompactOverlay::Notification(
                &self.notification_indicator,
            ))
        } else if self.volume_indicator.is_visible() {
            Some(ActiveCompactOverlay::Volume(&self.volume_indicator))
        } else {
            None
        }
    }
}
