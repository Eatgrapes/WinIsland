pub fn calculate_blur_sigmas(vel_w: f32, vel_h: f32, vel_view: f32, current_w: f32) -> (f32, f32) {
    let view_px_vel = vel_view.abs() * current_w;
    let sx = (vel_w.abs() * 0.3 + view_px_vel * 0.4).min(12.0);
    let sy = (vel_h.abs() * 0.3).min(10.0);
    (sx, sy)
}
