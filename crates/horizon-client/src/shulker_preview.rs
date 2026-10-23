//! The shulker box preview extra (`shulkerPreview`, `docs/extras.md`): a
//! shulker box's tooltip shows its 27 slots as a grid of items, tinted with
//! the box's dye colour, in place of vanilla's "count x name" lines.
use crate::{hud::rgba, items::ItemStack};

/// The grid's size in GUI pixels: 9x3 slots of 18 pixels and a 1-pixel frame.
pub const SIZE: (f32, f32) = (9.0 * 18.0 + 2.0, 3.0 * 18.0 + 2.0);

/// The 27 slots of a shulker box that holds something, by slot index.
pub fn grid(stack: &ItemStack) -> Option<Vec<Option<ItemStack>>> {
    if !stack.item.ends_with("shulker_box") {
        return None;
    }
    let contents = &stack.data()?.container;
    if contents.is_empty() {
        return None;
    }
    let mut slots = vec![None; 27];
    for (slot, item) in contents {
        if let Some(cell) = slots.get_mut(*slot as usize) {
            *cell = Some(item.clone());
        }
    }
    Some(slots)
}

/// Where slot `i`'s 18-pixel cell starts inside the grid.
pub fn slot_origin(i: usize) -> (f32, f32) {
    (1.0 + (i % 9) as f32 * 18.0, 1.0 + (i / 9) as f32 * 18.0)
}

/// The (slot, frame) colours for a shulker box item: its dye colour, the
/// undyed box purple.
pub fn colors(item: &str) -> (u32, u32) {
    let dye = item
        .trim_start_matches("minecraft:")
        .strip_suffix("_shulker_box")
        .unwrap_or("purple");
    let rgb: u32 = match dye {
        "white" => 0xF9FFFE,
        "orange" => 0xF9801D,
        "magenta" => 0xC74EBD,
        "light_blue" => 0x3AB3DA,
        "yellow" => 0xFED83D,
        "lime" => 0x80C71F,
        "pink" => 0xF38BAA,
        "gray" => 0x474F52,
        "light_gray" => 0x9D9D97,
        "cyan" => 0x169C9C,
        "blue" => 0x3C44AA,
        "brown" => 0x835432,
        "green" => 0x5E7C16,
        "red" => 0xB02E26,
        "black" => 0x1D1D21,
        _ => 0x8932B8,
    };
    let [r, g, b] = [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8];
    (rgba(r, g, b, 110), rgba(r / 2, g / 2, b / 2, 200))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(item: &str, contents: &[(u8, &str, u32)]) -> ItemStack {
        let mut stack = ItemStack::new(item, 1);
        stack.data_mut().container = contents
            .iter()
            .map(|(slot, id, count)| (*slot, ItemStack::new(id, *count)))
            .collect();
        stack
    }

    #[test]
    fn a_filled_shulker_box_lays_its_items_out_by_slot() {
        let stack = boxed(
            "minecraft:red_shulker_box",
            &[(0, "minecraft:stone", 64), (26, "minecraft:diamond", 3)],
        );
        let slots = grid(&stack).unwrap();
        assert_eq!(slots.len(), 27);
        assert_eq!(slots[0].as_ref().map(|s| s.count), Some(64));
        assert_eq!(slots[26].as_ref().map(|s| &*s.item), Some("minecraft:diamond"));
        assert!(slots[1..26].iter().all(Option::is_none));
        assert_eq!(slot_origin(26), (1.0 + 8.0 * 18.0, 1.0 + 2.0 * 18.0));
    }

    #[test]
    fn empty_boxes_and_other_containers_keep_vanilla_lines() {
        assert!(grid(&ItemStack::new("minecraft:shulker_box", 1)).is_none());
        assert!(grid(&boxed("minecraft:chest", &[(0, "minecraft:stone", 1)])).is_none());
    }

    #[test]
    fn colours_follow_the_dye() {
        assert_ne!(colors("minecraft:red_shulker_box"), colors("minecraft:blue_shulker_box"));
        assert_eq!(colors("minecraft:shulker_box"), colors("minecraft:purple_shulker_box"));
    }
}
