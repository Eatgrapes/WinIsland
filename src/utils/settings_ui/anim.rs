pub struct SwitchAnimator {
    positions: Vec<f32>,
    targets: Vec<f32>,
}

impl SwitchAnimator {
    pub fn new(initial: &[bool]) -> Self {
        let positions: Vec<f32> = initial.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect();
        Self {
            targets: positions.clone(),
            positions,
        }
    }

    pub fn new_with_anims(source: &SwitchAnimator, indices: &[usize]) -> Self {
        let positions: Vec<f32> = indices.iter().map(|&i| source.get(i)).collect();
        let targets: Vec<f32> = indices.iter().map(|&i| source.targets.get(i).copied().unwrap_or(0.0)).collect();
        Self { positions, targets }
    }

    pub fn get(&self, idx: usize) -> f32 {
        self.positions.get(idx).copied().unwrap_or(0.0)
    }

    pub fn set_target(&mut self, idx: usize, on: bool) {
        if idx < self.targets.len() {
            self.targets[idx] = if on { 1.0 } else { 0.0 };
        }
    }

    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        for i in 0..self.positions.len() {
            let diff = self.targets[i] - self.positions[i];
            if diff.abs() > 0.01 {
                self.positions[i] += diff * 0.2;
                changed = true;
            }
        }
        changed
    }
}
