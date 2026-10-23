//! Horizon's own one-line explanations of the 26.3 enchantments, shown under
//! each enchantment in item tooltips when the `enchantDescriptions` extra is
//! on (`docs/extras.md`). Vanilla has no such text.

/// What `id` (`minecraft:sharpness` or `sharpness`) does, if Horizon knows it.
pub fn description(id: &str) -> Option<&'static str> {
    Some(match id.strip_prefix("minecraft:").unwrap_or(id) {
        "aqua_affinity" => "Mine at normal speed underwater.",
        "bane_of_arthropods" => "More damage to spiders, bees and other arthropods; slows them.",
        "binding_curse" => "Cannot be taken off once worn, except in creative.",
        "blast_protection" => "Less damage and knockback from explosions.",
        "breach" => "Mace hits ignore part of the target's armor.",
        "channeling" => "A thrown trident calls lightning on its target during storms.",
        "density" => "Mace smash attacks deal more damage per block fallen.",
        "depth_strider" => "Walk faster underwater.",
        "efficiency" => "Mine faster with the right tool.",
        "feather_falling" => "Less fall damage.",
        "fire_aspect" => "Sets the target on fire.",
        "fire_protection" => "Less fire damage and shorter burning.",
        "flame" => "Arrows set their target on fire.",
        "fortune" => "More drops from ores, crops and some other blocks.",
        "frost_walker" => "Freezes water under your feet into ice.",
        "impaling" => "More trident damage to mobs in water or rain.",
        "infinity" => "Shooting uses no arrows while you have at least one.",
        "knockback" => "Knocks the target back further.",
        "looting" => "Mobs drop more loot, and rare drops more often.",
        "loyalty" => "A thrown trident flies back to you.",
        "luck_of_the_sea" => "Better chance of treasure when fishing.",
        "lunge" => "A spear jab dashes you forward, at the cost of hunger.",
        "lure" => "Fish bite sooner.",
        "mending" => "Experience you collect repairs this item.",
        "multishot" => "Shoots three arrows at once for the cost of one.",
        "piercing" => "Arrows pass through several mobs.",
        "power" => "Arrows deal more damage.",
        "projectile_protection" => "Less damage from arrows and other projectiles.",
        "protection" => "Less damage from most sources.",
        "punch" => "Arrows knock the target back further.",
        "quick_charge" => "Loads the crossbow faster.",
        "respiration" => "Hold your breath longer underwater.",
        "riptide" => "Throwing the trident in water or rain launches you with it.",
        "sharpness" => "More melee damage.",
        "silk_touch" => "Mined blocks drop themselves.",
        "smite" => "More damage to zombies, skeletons and other undead.",
        "soul_speed" => "Walk faster on soul sand and soul soil.",
        "sweeping_edge" => "Sweep attacks hit nearby mobs harder.",
        "swift_sneak" => "Move faster while sneaking.",
        "thorns" => "Attackers take damage, at a cost in durability.",
        "unbreaking" => "The item loses durability less often.",
        "vanishing_curse" => "Disappears when you die.",
        "wind_burst" => "A mace smash launches you back into the air.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_26_3_enchantment_has_a_description() {
        let ids = [
            "aqua_affinity", "bane_of_arthropods", "binding_curse", "blast_protection", "breach",
            "channeling", "density", "depth_strider", "efficiency", "feather_falling",
            "fire_aspect", "fire_protection", "flame", "fortune", "frost_walker", "impaling",
            "infinity", "knockback", "looting", "loyalty", "luck_of_the_sea", "lunge", "lure",
            "mending", "multishot", "piercing", "power", "projectile_protection", "protection",
            "punch", "quick_charge", "respiration", "riptide", "sharpness", "silk_touch", "smite",
            "soul_speed", "sweeping_edge", "swift_sneak", "thorns", "unbreaking",
            "vanishing_curse", "wind_burst",
        ];
        for id in ids {
            assert!(description(&format!("minecraft:{id}")).is_some(), "{id}");
        }
        assert_eq!(description("sharpness"), Some("More melee damage."));
        assert_eq!(description("minecraft:unknown"), None);
    }
}
