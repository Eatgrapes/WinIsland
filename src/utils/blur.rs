use crate::utils::gpu::{GpuProfile, gpu_profile};

pub fn calculate_blur_sigmas(vel_w: f32, vel_h: f32, vel_view: f32, current_w: f32) -> (f32, f32) {
    let (max_sx, max_sy) = if gpu_profile() == GpuProfile::Integrated {
        (4.0, 3.5)
    } else {
        (12.0, 10.0)
    };
    let view_px_vel = vel_view.abs() * current_w;
    let sx = (vel_w.abs() * 0.3 + view_px_vel * 0.4).min(max_sx);
    let sy = (vel_h.abs() * 0.3).min(max_sy);
    (sx, sy)
}
