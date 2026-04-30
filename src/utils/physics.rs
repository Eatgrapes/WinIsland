pub struct Spring {
    pub value: f32,
    pub velocity: f32,
}
impl Spring {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }
    pub fn update_dt(&mut self, target: f32, stiffness: f32, damping: f32, dt: f32) {
        let force = (target - self.value) * stiffness * dt;
        self.velocity = (self.velocity + force) * damping.powf(dt);
        self.value += self.velocity * dt;
    }
}
