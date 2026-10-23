//! The Item Highlighter extra: the slots that hold a newly gained item get a
//! mark until the player looks at them (the cursor over the slot in the
//! inventory screen) or picks them (the hotbar's selection).
//!
//! The player's inventory is compared with itself: an item is new when the
//! number of that item in the whole inventory grew. A stack the player moves
//! between slots does not change that number, so it is not new, and neither is
//! a stack that was already in the inventory. Items the player crafted are
//! not new either.
//!
//! The state is not kept between sessions: a world that is loaded comes with
//! its items, and those are not new.

use crate::items::ItemStack;
use std::collections::HashMap;
use std::sync::Arc;

/// The player's inventory: the hotbar (0..8) and the rest (9..35).
pub const INVENTORY: usize = 36;
/// The armour slots: head, chest, legs, feet.
pub const ARMOUR: usize = 4;
/// The off hand.
pub const OFFHAND: usize = 1;
/// Every slot the highlighter watches.
pub const SLOTS: usize = INVENTORY + ARMOUR + OFFHAND;

/// The watched slot of the armour slot `n` (0 = head).
pub const fn armour(n: usize) -> usize {
    INVENTORY + n
}
/// The watched slot of the off hand.
pub const OFFHAND_SLOT: usize = INVENTORY + ARMOUR;

/// How long the highlighter waits after the world was loaded before it marks
/// anything: the items a world (or a server) comes with are not new.
pub const WARMUP_MS: u64 = 2000;
/// How long a crafted item is not new.
pub const CRAFTED_MS: u64 = 2000;

/// What makes two stacks the same item for the count: the id and the damage.
/// The components are left out, so two potions of a kind are one item.
type Key = (Arc<str>, u32);

fn key(stack: &ItemStack) -> Key {
    (stack.item.clone(), stack.damage)
}

/// The number of every item the player has: the slots and the stack on the
/// cursor. The cursor counts too, so a stack the player moves from a slot to
/// another one (through the cursor) does not look like a new item.
fn counts<'a>(
    slots: impl Iterator<Item = &'a Option<ItemStack>>,
    carried: Option<&ItemStack>,
) -> HashMap<Key, u32> {
    let mut counts: HashMap<Key, u32> = HashMap::new();
    for stack in slots.filter_map(|s| s.as_ref()).chain(carried) {
        *counts.entry(key(stack)).or_default() += stack.count;
    }
    counts
}

/// Two stacks in the same slot: the same item with the same count.
fn same_slot(a: &Option<ItemStack>, b: &Option<ItemStack>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_item_same_components(b) && a.count == b.count,
        _ => false,
    }
}

/// The highlighter's state: the last seen inventory and the marked slots.
#[derive(Clone, Debug)]
pub struct Highlighter {
    /// The inventory as it was at the last update.
    snapshot: Vec<Option<ItemStack>>,
    /// The stack on the cursor as it was at the last update.
    carried: Option<ItemStack>,
    /// The slots holding a newly gained item.
    marked: Vec<bool>,
    /// The items the player crafted: they are not new.
    crafted: Vec<(Arc<str>, u64)>,
    /// When the world was loaded (milliseconds since the game started).
    since: u64,
    /// The hotbar slot the player picked last.
    selected: Option<usize>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    /// A highlighter that has seen nothing yet.
    pub fn new() -> Self {
        Self {
            snapshot: Vec::new(),
            carried: None,
            marked: vec![false; SLOTS],
            crafted: Vec::new(),
            since: 0,
            selected: None,
        }
    }

    /// The world was loaded (or the player joined a server): nothing in the
    /// inventory is new.
    pub fn reset(&mut self, now: u64) {
        self.snapshot.clear();
        self.carried = None;
        self.marked.fill(false);
        self.crafted.clear();
        self.since = now;
        self.selected = None;
    }

    /// Does the slot hold a newly gained item?
    pub fn marked(&self, slot: usize) -> bool {
        self.marked.get(slot).copied().unwrap_or(false)
    }

    /// The player saw the item: the mark is gone.
    pub fn clear(&mut self, slot: usize) {
        if let Some(m) = self.marked.get_mut(slot) {
            *m = false;
        }
    }

    /// The watched slot of the game's slot `slot` (the inventory screen's
    /// ids): None for the slots the highlighter does not watch.
    pub fn watched(slot: usize) -> Option<usize> {
        match slot {
            0..=35 => Some(slot),
            500..=503 => Some(armour(slot - 500)),
            504 => Some(OFFHAND_SLOT),
            _ => None,
        }
    }

    /// The hotbar slot the player picked last.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// The player picked the hotbar slot: the item in it is not new any more.
    pub fn note_selected(&mut self, slot: usize) {
        self.selected = Some(slot);
        self.clear(slot);
    }

    /// The player crafted the item: it is not new.
    pub fn note_crafted(&mut self, item: &str, now: u64) {
        self.crafted
            .retain(|&(_, at)| now.saturating_sub(at) < CRAFTED_MS);
        self.crafted.push((Arc::from(item), now));
    }

    /// The item was crafted: it is not new.
    fn is_crafted(&self, item: &str, now: u64) -> bool {
        self.crafted
            .iter()
            .any(|(id, at)| &**id == item && now.saturating_sub(*at) < CRAFTED_MS)
    }

    /// Did the item come into the inventory?
    ///
    /// The number of the item in the whole inventory grew: a pickup, a stack
    /// that grew, a crafted item (which the caller knows about). A stack the
    /// player moves between slots does not change the number.
    fn gained(
        &self,
        before: &HashMap<Key, u32>,
        after: &HashMap<Key, u32>,
        stack: &ItemStack,
        now: u64,
    ) -> bool {
        if self.is_crafted(&stack.item, now) {
            return false;
        }
        let k = key(stack);
        after.get(&k).copied().unwrap_or(0) > before.get(&k).copied().unwrap_or(0)
    }

    /// Compares the inventory with the last one and marks the slots that got
    /// a new item. The slots are the inventory, the armour and the off hand;
    /// `carried` is the stack on the cursor.
    pub fn update<'a, I>(&mut self, slots: I, carried: Option<&ItemStack>, now: u64)
    where
        I: Iterator<Item = &'a Option<ItemStack>> + Clone,
    {
        if slots.clone().count() != SLOTS {
            return;
        }
        // The first look at the inventory, and the world still loading: the
        // items it comes with are not new.
        if self.snapshot.len() != SLOTS || now.saturating_sub(self.since) < WARMUP_MS {
            self.snapshot = slots.cloned().collect();
            return;
        }
        // Nothing changed: the inventory is watched, but there is nothing to
        // do (and the snapshot stays as it is).
        if self
            .snapshot
            .iter()
            .zip(slots.clone())
            .all(|(old, new)| same_slot(old, new))
            && self.carried.as_ref() == carried
        {
            return;
        }
        let before = counts(self.snapshot.iter(), self.carried.as_ref());
        let after = counts(slots.clone(), carried);
        let mut marks = self.marked.clone();
        for (slot, (old, new)) in self.snapshot.iter().zip(slots.clone()).enumerate() {
            match (old, new) {
                (None, None) => {}
                // The item is gone: the mark goes with it.
                (_, None) => set_mark(&mut marks, slot, false),
                (None, Some(stack)) => {
                    if self.gained(&before, &after, stack, now) {
                        set_mark(&mut marks, slot, true);
                    }
                }
                (Some(old), Some(new)) => {
                    if old.same_item_same_components(new) {
                        // More of the same stack: a pickup grew it.
                        if new.count > old.count && self.gained(&before, &after, new, now) {
                            set_mark(&mut marks, slot, true);
                        }
                    } else if self.gained(&before, &after, new, now) {
                        set_mark(&mut marks, slot, true);
                    } else {
                        // Another item took the slot: that one is not new.
                        set_mark(&mut marks, slot, false);
                    }
                }
            }
        }
        self.marked = marks;
        self.snapshot = slots.cloned().collect();
        self.carried = carried.cloned();
    }
}

/// The slot holds a new item (or does not).
fn set_mark(marks: &mut [bool], slot: usize, marked: bool) {
    if let Some(m) = marks.get_mut(slot) {
        *m = marked;
    }
}

/// The frame's alpha: a slow pulse.
pub fn pulse(now_ms: u64) -> u8 {
    let phase = (now_ms % 2000) as f32 / 2000.0;
    let wave = (phase * std::f32::consts::TAU).sin();
    (110.0 + 80.0 * wave).clamp(0.0, 255.0) as u8
}

/// The mark of a new item: a soft frame around the slot and a dot in its
/// top right corner.
pub fn draw_mark(
    batch: &mut crate::hud::HudBatch,
    atlas: &crate::hud::HudAtlas,
    x: f32,
    y: f32,
    now_ms: u64,
) {
    let frame = crate::hud::rgba(255, 208, 88, pulse(now_ms));
    let dot = crate::hud::rgba(255, 240, 168, 220);
    // A one pixel frame around the 16x16 slot.
    batch.fill(atlas, x, y, 16.0, 1.0, frame);
    batch.fill(atlas, x, y + 15.0, 16.0, 1.0, frame);
    batch.fill(atlas, x, y, 1.0, 16.0, frame);
    batch.fill(atlas, x + 15.0, y, 1.0, 16.0, frame);
    // The dot in the corner.
    batch.fill(atlas, x + 12.0, y + 1.0, 3.0, 3.0, dot);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u32) -> Option<ItemStack> {
        Some(ItemStack::new(item, count))
    }

    /// An inventory of `SLOTS` empty slots.
    fn empty() -> Vec<Option<ItemStack>> {
        vec![None; SLOTS]
    }

    fn slots(items: &[(usize, &str, u32)]) -> Vec<Option<ItemStack>> {
        let mut v = empty();
        for &(slot, item, count) in items {
            v[slot] = stack(item, count);
        }
        v
    }

    /// The highlighter, past its warmup.
    fn warm() -> Highlighter {
        let mut h = Highlighter::new();
        h.reset(0);
        let items = empty();
        h.update(items.iter(), None, WARMUP_MS);
        h
    }

    #[test]
    fn a_picked_up_item_is_marked() {
        let mut h = warm();
        let items = slots(&[(0, "minecraft:dirt", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
        assert!(!h.marked(1));
    }

    #[test]
    fn the_first_inventory_is_not_new() {
        let mut h = Highlighter::new();
        h.reset(0);
        // The world comes with its items: they are not new.
        let items = slots(&[(0, "minecraft:dirt", 1), (5, "minecraft:stone", 64)]);
        h.update(items.iter(), None, 10);
        assert!(!h.marked(0));
        assert!(!h.marked(5));
    }

    #[test]
    fn the_warmup_keeps_the_worlds_items_from_being_new() {
        let mut h = Highlighter::new();
        h.reset(0);
        let mut items = empty();
        h.update(items.iter(), None, 10);
        items[3] = stack("minecraft:diamond", 1);
        h.update(items.iter(), None, WARMUP_MS - 1);
        assert!(!h.marked(3));
    }

    #[test]
    fn a_stack_moved_to_another_slot_is_not_new() {
        let mut h = warm();
        let mut items = slots(&[(0, "minecraft:dirt", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
        // The player moves the stack to another slot: the number of dirt in
        // the inventory did not grow.
        items[0] = None;
        items[9] = stack("minecraft:dirt", 1);
        h.update(items.iter(), None, WARMUP_MS + 20);
        assert!(!h.marked(9));
    }

    #[test]
    fn a_stack_moved_through_the_cursor_is_not_new() {
        let mut h = warm();
        let mut items = slots(&[(0, "minecraft:dirt", 3)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        // The player picks the stack up: it is on the cursor, not in a slot.
        let carried = ItemStack::new("minecraft:dirt", 3);
        items[0] = None;
        h.update(items.iter(), Some(&carried), WARMUP_MS + 20);
        assert!(!h.marked(0));
        // And puts it in another slot.
        items[5] = Some(carried);
        h.update(items.iter(), None, WARMUP_MS + 30);
        assert!(!h.marked(5));
    }

    #[test]
    fn a_split_stack_is_not_new() {
        let mut h = warm();
        let mut items = slots(&[(0, "minecraft:dirt", 64)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
        // The player splits the stack in two: the number of dirt in the
        // inventory did not grow, so the new stack is not new.
        items[0] = stack("minecraft:dirt", 32);
        items[1] = stack("minecraft:dirt", 32);
        h.update(items.iter(), None, WARMUP_MS + 20);
        assert!(!h.marked(1));
    }

    #[test]
    fn more_of_a_stack_the_player_has_is_new() {
        let mut h = warm();
        let mut items = slots(&[(0, "minecraft:dirt", 60)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        // A pickup grows the stack.
        items[0] = stack("minecraft:dirt", 64);
        h.update(items.iter(), None, WARMUP_MS + 20);
        assert!(h.marked(0));
    }

    #[test]
    fn a_dropped_item_loses_its_mark() {
        let mut h = warm();
        let mut items = slots(&[(0, "minecraft:dirt", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
        // The player drops it.
        items[0] = None;
        h.update(items.iter(), None, WARMUP_MS + 20);
        assert!(!h.marked(0));
    }

    #[test]
    fn an_item_that_comes_into_the_inventory_is_new() {
        let mut h = warm();
        let mut items = slots(&[(0, "minecraft:dirt", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
        // Another item takes the slot: it came into the inventory.
        items[0] = stack("minecraft:stone", 1);
        h.update(items.iter(), None, WARMUP_MS + 20);
        assert!(h.marked(0));
    }

    #[test]
    fn a_swap_of_two_stacks_is_not_new() {
        let mut h = warm();
        let mut items = slots(&[(0, "minecraft:dirt", 1), (1, "minecraft:stone", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
        assert!(h.marked(1));
        // The player swaps the two stacks: neither came into the inventory.
        items[0] = stack("minecraft:stone", 1);
        items[1] = stack("minecraft:dirt", 1);
        h.update(items.iter(), None, WARMUP_MS + 20);
        assert!(!h.marked(0));
        assert!(!h.marked(1));
    }

    #[test]
    fn the_armour_and_the_off_hand_are_watched() {
        let mut h = warm();
        let mut items = empty();
        items[armour(0)] = stack("minecraft:iron_helmet", 1);
        items[OFFHAND_SLOT] = stack("minecraft:shield", 1);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(armour(0)));
        assert!(h.marked(OFFHAND_SLOT));
        assert!(!h.marked(armour(1)));
    }

    #[test]
    fn a_crafted_item_is_not_new() {
        let mut h = warm();
        h.note_crafted("minecraft:stick", WARMUP_MS + 5);
        let items = slots(&[(0, "minecraft:stick", 4)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(!h.marked(0));
    }

    #[test]
    fn the_player_looking_at_a_slot_clears_its_mark() {
        let mut h = warm();
        let items = slots(&[(0, "minecraft:dirt", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
        h.clear(0);
        assert!(!h.marked(0));
    }

    #[test]
    fn the_player_picking_a_hotbar_slot_clears_its_mark() {
        let mut h = warm();
        let items = slots(&[(2, "minecraft:dirt", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(2));
        h.note_selected(2);
        assert!(!h.marked(2));
        assert_eq!(h.selected(), Some(2));
    }

    #[test]
    fn the_game_slots_map_to_the_watched_ones() {
        assert_eq!(Highlighter::watched(0), Some(0));
        assert_eq!(Highlighter::watched(35), Some(35));
        assert_eq!(Highlighter::watched(500), Some(armour(0)));
        assert_eq!(Highlighter::watched(503), Some(armour(3)));
        assert_eq!(Highlighter::watched(504), Some(OFFHAND_SLOT));
        // The crafting grid and the containers are not watched.
        assert_eq!(Highlighter::watched(100), None);
        assert_eq!(Highlighter::watched(200), None);
        assert_eq!(Highlighter::watched(300), None);
    }

    #[test]
    fn a_default_highlighter_watches_every_slot() {
        let mut h = Highlighter::default();
        h.reset(0);
        let items = empty();
        h.update(items.iter(), None, WARMUP_MS);
        let items = slots(&[(0, "minecraft:dirt", 1)]);
        h.update(items.iter(), None, WARMUP_MS + 10);
        assert!(h.marked(0));
    }

    #[test]
    fn the_mark_pulses_between_two_alphas() {
        let low = pulse(0);
        let high = pulse(500);
        assert!(high > low);
        assert!(pulse(1000) <= 200);
    }
}
