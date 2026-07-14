mod volume;

use skia_safe::{Canvas, Rect};

use self::volume::{VolumeIndicator, VolumeMonitor};

#[derive(Clone, Copy)]
pub struct CompactSize {
    pub width: f32,
    pub height: f32,
}

pub struct CompactOverlay {
    volume_monitor: VolumeMonitor,
    volume_indicator: VolumeIndicator,
}

impl Default for CompactOverlay {
    fn default() -> Self {
        Self {
            volume_monitor: VolumeMonitor::new(),
            volume_indicator: VolumeIndicator::default(),
        }
    }
}

impl CompactOverlay {
    pub fn update(&mut self, can_present: bool) {
        self.volume_indicator
            .update(self.volume_monitor.snapshot(), can_present);
    }

    pub fn is_visible(&self) -> bool {
        self.volume_indicator.is_visible()
    }

    pub fn target_size(
        &self,
        base_width: f32,
        base_height: f32,
        scale: f32,
    ) -> Option<CompactSize> {
        self.volume_indicator.is_visible().then(|| {
            self.volume_indicator
                .target_size(base_width, base_height, scale)
        })
    }

    pub fn draw(&self, canvas: &Canvas, rect: Rect, scale: f32, alpha: f32) {
        if self.volume_indicator.is_visible() {
            self.volume_indicator.draw(canvas, rect, scale, alpha);
        }
    }
}
