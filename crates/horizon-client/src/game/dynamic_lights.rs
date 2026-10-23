//! The Dynamic lights extra (`horizon_client::dynamic_lights`): what gives
//! off light this frame. The player's and mobs' held items, mobs that glow
//! by kind, burning mobs and players, and dropped items.
use super::Game;
use glam::DVec3;
use horizon_client::dynamic_lights::{self, BURNING, Source};

impl Game<'_> {
    /// The light an item gives off held or dropped: its block's emission,
    /// else its own (`dynamic_lights::item_level`).
    fn dynamic_item_light(&self, id: &str) -> u8 {
        let block = self
            .pack
            .assets
            .registry
            .default_state(id)
            .map_or(0, |s| self.res.facts().get(s).emission.min(15));
        block.max(dynamic_lights::item_level(id))
    }

    /// The dynamic lights the shaders get this frame, camera relative;
    /// empty with the extra off. Fast gathers the sources every few frames
    /// (they stay put in the world in between); both pack them every frame.
    pub(super) fn dynamic_lights(
        &mut self,
        camera: DVec3,
        alpha: f64,
        reach: f64,
    ) -> Vec<[f32; 4]> {
        let mode = self.settings.extras.dynamic_lights;
        if mode == dynamic_lights::Mode::Off {
            self.dynamic_light_sources.clear();
            return Vec::new();
        }
        if mode.refresh(self.frames) {
            self.dynamic_light_sources = self.dynamic_light_sources(alpha);
        }
        dynamic_lights::pack(&self.dynamic_light_sources, camera, reach)
    }

    fn dynamic_light_sources(&self, alpha: f64) -> Vec<Source> {
        let mut sources = Vec::new();
        if let Some(p) = &self.player
            && !p.spectator
        {
            let feet = self.previous_position.lerp(p.position, alpha);
            let held = [
                self.inventory.get(self.selected),
                self.inventory.get(super::shield::OFFHAND_SLOT),
            ]
            .into_iter()
            .flatten()
            .flatten()
            .map(|s| self.dynamic_item_light(&s.item))
            .max()
            .unwrap_or(0);
            let level = if p.fire_ticks > 0 {
                BURNING.max(held)
            } else {
                held
            };
            sources.push(Source {
                pos: DVec3::new(feet.x, feet.y + f64::from(p.dimensions.eye_height), feet.z),
                level,
            });
        }
        for m in self.mobs.iter().filter(|m| !m.removed) {
            let held = [&m.extra.main_hand, &m.extra.off_hand]
                .into_iter()
                .flatten()
                .map(|id| self.dynamic_item_light(id))
                .max()
                .unwrap_or(0);
            let mut level = held.max(dynamic_lights::entity_level(&m.kind.id));
            if m.fire_ticks > 0 {
                level = BURNING;
            }
            if level > 0 {
                let at = m.previous.lerp(m.position, alpha);
                sources.push(Source {
                    pos: DVec3::new(at.x, at.y + m.kind.height * 0.5, at.z),
                    level,
                });
            }
        }
        for item in &self.dropped {
            let level = if item.fire_ticks > 0 {
                BURNING
            } else {
                self.dynamic_item_light(&item.stack.item)
            };
            if level > 0 {
                let at = item.previous.lerp(item.position, alpha);
                sources.push(Source {
                    pos: DVec3::new(at.x, at.y + 0.25, at.z),
                    level,
                });
            }
        }
        sources
    }
}
