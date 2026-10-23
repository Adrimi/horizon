//! The dynamic lights the shaders get this frame: the sources the mods
//! gave (`lighting`, API 0.1.13), packed camera-relative. With no mod on
//! the list is empty and the picture is vanilla's.
use super::Game;
use glam::DVec3;
use horizon_client::dynamic_lights::{self, Source};

impl Game<'_> {
    /// The dynamic lights the shaders get this frame, camera relative;
    /// empty when no mod gives any.
    pub(super) fn dynamic_lights(&mut self, camera: DVec3, reach: f64) -> Vec<[f32; 4]> {
        let mods: Vec<Source> = self
            .mods
            .mods
            .lights()
            .into_iter()
            .map(|([x, y, z], level)| Source {
                pos: DVec3::new(x, y, z),
                level,
            })
            .collect();
        dynamic_lights::pack(&mods, camera, reach)
    }
}
