//! All world/camera math remains f64. Screen positions are logical pixels.
#[derive(Debug, Clone)]
pub struct Viewport {
    pub center: [f64; 2],
    pub scale: f64,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            scale: 4.0,
        }
    }
}
impl Viewport {
    pub fn world(&self, screen: [f64; 2], size: [f64; 2]) -> [f64; 2] {
        [
            self.center[0] + (screen[0] - size[0] * 0.5) / self.scale,
            self.center[1] - (screen[1] - size[1] * 0.5) / self.scale,
        ]
    }
    pub fn screen(&self, world: [f64; 2], size: [f64; 2]) -> [f64; 2] {
        [
            (world[0] - self.center[0]) * self.scale + size[0] * 0.5,
            -(world[1] - self.center[1]) * self.scale + size[1] * 0.5,
        ]
    }
    pub fn pan(&mut self, delta: [f64; 2]) {
        self.center[0] -= delta[0] / self.scale;
        self.center[1] += delta[1] / self.scale;
    }
    pub fn zoom(&mut self, factor: f64, anchor: [f64; 2], size: [f64; 2]) {
        let before = self.world(anchor, size);
        self.scale = (self.scale * factor).clamp(1e-5, 1e7);
        let after = self.world(anchor, size);
        for i in 0..2 {
            self.center[i] += before[i] - after[i];
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_keeps_point_under_pointer() {
        let mut v = Viewport::default();
        let p = [92.0, 611.0];
        let size = [1200.0, 800.0];
        let before = v.world(p, size);
        v.zoom(2.3, p, size);
        let after = v.world(p, size);
        assert!((before[0] - after[0]).abs() < 1e-10);
        assert!((before[1] - after[1]).abs() < 1e-10);
    }
    #[test]
    fn large_origin_round_trip_and_pan() {
        let mut v = Viewport {
            center: [1e9, -1e9],
            scale: 1000.0,
        };
        let p = [1e9 + 0.125, -1e9 + 0.25];
        assert_eq!(v.world(v.screen(p, [800.0, 600.0]), [800.0, 600.0]), p);
        v.pan([100.0, -50.0]);
        assert!((v.center[0] - (1e9 - 0.1)).abs() < 1e-6);
    }
}
