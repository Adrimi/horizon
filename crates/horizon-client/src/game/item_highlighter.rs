//! The Item Highlighter extra in the game: it watches the player's inventory
//! and marks the slots that hold a newly gained item, in the hotbar and in
//! the inventory screen.
use super::Game;
use horizon_client::{
    hud::HudBatch,
    item_highlighter::{self, Highlighter},
};

impl Game<'_> {
    /// The time the highlighter works with: the milliseconds since the game
    /// started.
    fn highlighter_now(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    /// Compares the player's inventory with the last one and marks the slots
    /// that got a new item. Nothing happens with the extra off.
    pub(super) fn update_highlighter(&mut self) {
        if !self.settings.extras.item_highlighter {
            return;
        }
        let now = self.highlighter_now();
        // The hotbar slot the player picked: the item in it is not new any
        // more.
        if self.highlighter.selected() != Some(self.selected) {
            self.highlighter.note_selected(self.selected);
        }
        // The item the player crafted is not new: the result is known while
        // the grid still holds the ingredients.
        if self.craft_grid.iter().any(|c| c.is_some())
            && let Some(result) = self.craft_result()
        {
            self.highlighter.note_crafted(&result.item, now);
        }
        // The inventory, the armour and the off hand; the stack on the
        // cursor counts too, so a stack the player moves does not look new.
        let inventory = self.inventory.iter();
        let armour = self.armor.iter();
        let offhand = std::iter::once(&self.offhand);
        let carried = self.carried.as_ref();
        self.highlighter
            .update(inventory.chain(armour).chain(offhand), carried, now);
    }

    /// The player looked at a slot of the inventory screen: the mark of a
    /// new item there is gone.
    pub(super) fn clear_highlight(&mut self, slot: usize) {
        if let Some(watched) = Highlighter::watched(slot) {
            self.highlighter.clear(watched);
        }
    }

    /// The marks of the hotbar's slots that hold a new item.
    pub(super) fn draw_hotbar_marks(&mut self, batch: &mut HudBatch, w: i32, h: i32) {
        if !self.settings.extras.item_highlighter {
            return;
        }
        let now = self.highlighter_now();
        let center = w / 2;
        for slot in 0..9 {
            if !self.highlighter.marked(slot) {
                continue;
            }
            // The item's place in the hotbar (Hud.extractItemHotbar).
            let x = (center - 90 + slot as i32 * 20 + 2) as f32;
            let y = (h - 16 - 3) as f32;
            item_highlighter::draw_mark(batch, &self.gui.atlas, x, y, now);
        }
    }

    /// The marks of the inventory screen: the player's slots, the armour and
    /// the off hand.
    pub(super) fn draw_inventory_marks(&mut self, batch: &mut HudBatch, ox: f32, oy: f32) {
        if !self.settings.extras.item_highlighter {
            return;
        }
        let now = self.highlighter_now();
        for slot in 0..36 {
            if !self.highlighter.marked(slot) {
                continue;
            }
            let (sx, sy) = self.slot_position(slot);
            item_highlighter::draw_mark(batch, &self.gui.atlas, ox + sx, oy + sy, now);
        }
        for n in 0..4 {
            if !self.highlighter.marked(item_highlighter::armour(n)) {
                continue;
            }
            let (sx, sy) = self.slot_position(500 + n);
            item_highlighter::draw_mark(batch, &self.gui.atlas, ox + sx, oy + sy, now);
        }
        if self.highlighter.marked(item_highlighter::OFFHAND_SLOT) {
            let (sx, sy) = self.slot_position(504);
            item_highlighter::draw_mark(batch, &self.gui.atlas, ox + sx, oy + sy, now);
        }
    }

    /// The marks of the hotbar's row in the creative screen's item tab.
    pub(super) fn draw_creative_hotbar_marks(&mut self, batch: &mut HudBatch, ox: f32, oy: f32) {
        if !self.settings.extras.item_highlighter {
            return;
        }
        let now = self.highlighter_now();
        for slot in 0..9 {
            if !self.highlighter.marked(slot) {
                continue;
            }
            let x = ox + 9.0 + slot as f32 * 18.0;
            let y = oy + 112.0;
            item_highlighter::draw_mark(batch, &self.gui.atlas, x, y, now);
        }
    }
}
