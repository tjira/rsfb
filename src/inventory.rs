use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use strum::IntoEnumIterator;

use sf_api::{
    command::{AttributeType, BlacksmithAction, Command},
    gamestate::character::Class,
    gamestate::dungeons::CompanionClass,
    gamestate::items::{BlacksmithPayment, EquipmentSlot, GemSlot, GemType, Item},
    gamestate::items::{ItemCommandIdent, ItemPosition, ItemType},
    gamestate::items::{PlayerItemPosition, PotionType},
    session::SimpleSession,
};

use crate::log::log;

fn should_equip(session: &SimpleSession, it: &Item, slot: EquipmentSlot) -> bool {
    let Some(gs) = session.game_state() else {
        return false;
    };

    let attr = gs.character.class.main_attribute();

    if !it.can_be_equipped_by(gs.character.class) {
        return false;
    }

    let Some(eq) = gs.character.equipment.0[slot].as_ref() else {
        return true;
    };

    let (a_old, a_new) = (eq.attributes[attr] as f64, it.attributes[attr] as f64);

    let new_is_special = it.is_epic() || it.is_legendary();
    let old_is_special = eq.is_epic() || eq.is_legendary();

    if !new_is_special && old_is_special {
        return a_new > a_old * crate::constant::EPIC_LEGENDARY_MULTIPLIER;
    }

    if new_is_special && !old_is_special {
        return a_old < a_new * crate::constant::EPIC_LEGENDARY_MULTIPLIER;
    }

    a_new > a_old
}

fn s_eq_comp(session: &SimpleSession, it: &Item, slot: EquipmentSlot, cc: CompanionClass) -> bool {
    let Some(gs) = session.game_state() else {
        return false;
    };

    if !gs.dungeons.can_companion_equip(cc, it) {
        return false;
    }

    let Some(ref companions) = gs.dungeons.companions else {
        return false;
    };

    let attr = Class::from(cc).main_attribute();

    let Some(eq) = companions[cc].equipment.0[slot].as_ref() else {
        return true;
    };

    let (a_old, a_new) = (eq.attributes[attr] as f64, it.attributes[attr] as f64);

    let new_is_special = it.is_epic() || it.is_legendary();
    let old_is_special = eq.is_epic() || eq.is_legendary();

    if !new_is_special && old_is_special {
        return a_new > a_old * crate::constant::EPIC_LEGENDARY_MULTIPLIER;
    }

    if new_is_special && !old_is_special {
        return a_old < a_new * crate::constant::EPIC_LEGENDARY_MULTIPLIER;
    }

    a_new > a_old
}

fn target_equipment_slot(session: &SimpleSession, it: &Item) -> Option<EquipmentSlot> {
    let gs = session.game_state()?;
    let slot = it.typ.equipment_slot()?;

    if gs.character.class == Class::Assassin && it.typ.is_weapon() {
        if should_equip(session, it, EquipmentSlot::Weapon) {
            return Some(EquipmentSlot::Weapon);
        }

        if should_equip(session, it, EquipmentSlot::Shield) {
            return Some(EquipmentSlot::Shield);
        }

        return None;
    }

    if should_equip(session, it, slot) {
        return Some(slot);
    }

    None
}

pub(crate) fn is_equippable(session: &SimpleSession, item: &Item) -> bool {
    if target_equipment_slot(session, item).is_some() {
        return true;
    }

    let Some(slot) = item.typ.equipment_slot() else {
        return false;
    };

    for companion in [CompanionClass::Warrior, CompanionClass::Mage, CompanionClass::Scout] {
        if s_eq_comp(session, item, slot, companion) {
            return true;
        }
    }

    false
}

fn sell(s: &SimpleSession, pos: PlayerItemPosition, ii: ItemCommandIdent, item: &Item) -> Command {
    let Some(gs) = s.game_state() else {
        return Command::SellShop { item_pos: pos, item_ident: ii };
    };

    let toilet_unlocked = gs.character.level >= 100 && gs.tavern.toilet.is_some_and(|t| t.aura > 0);

    if toilet_unlocked {
        if let Some(toilet) = gs.tavern.toilet {
            if toilet.sacrifices_left > 0 {
                return Command::ToiletDrop { item_pos: pos };
            }
        }
    }

    if gs.character.level >= 90 {
        if let Some(blacksmith) = &gs.blacksmith {
            if blacksmith.dismantle_left > 0 && item.typ.equipment_slot().is_some() {
                let action = BlacksmithAction::Dismantle;

                return Command::Blacksmith { item_pos: pos, action, item_ident: ii };
            }
        }
    }

    let witch_unlocked = gs.character.level >= 66 && gs.witch.is_some();

    if witch_unlocked {
        if let Some(witch) = &gs.witch {
            if let Some(slot) = item.typ.equipment_slot() {
                if witch.required_item == Some(slot) {
                    return Command::WitchDropCauldron { item_pos: pos };
                }
            }
        }
    }

    Command::SellShop { item_pos: pos, item_ident: ii }
}

fn matches_attr(gem_typ: GemType, attr_typ: AttributeType) -> bool {
    if gem_typ == GemType::Strength && attr_typ == AttributeType::Strength {
        return true;
    }

    if gem_typ == GemType::Dexterity && attr_typ == AttributeType::Dexterity {
        return true;
    }

    if gem_typ == GemType::Intelligence && attr_typ == AttributeType::Intelligence {
        return true;
    }

    if gem_typ == GemType::Constitution && attr_typ == AttributeType::Constitution {
        return true;
    }

    if gem_typ == GemType::Luck && attr_typ == AttributeType::Luck {
        return true;
    }

    false
}

fn socket_costs(it: &Item) -> Option<BlacksmithPayment> {
    if it.gem_slot.is_some() || it.equipment_ident().is_none() {
        return None;
    }

    let item_stats = it.attributes.values().filter(|a| **a > 0).count();

    let mut price = f64::from(*it.attributes.values().max().unwrap_or(&0));

    if item_stats >= 4 {
        price *= 1.2;
    }

    if it.class.is_some_and(|a| a == Class::Scout || a == Class::Mage) && it.typ.is_weapon() {
        price /= 2.0;
    }

    if item_stats == 1 && price > 66.0 {
        price = (price * 0.75).ceil();
    }

    price = price.round().powf(1.2).floor();

    let metal = (price * 5.0).floor() as u64;

    let arcane_factor = match item_stats {
        0 => 0.25,
        1 => 0.25,
        2 => 0.50,
        _ => 1.00,
    };

    let arcane = 10.max(((price * arcane_factor).floor() as u64) * 10);

    Some(BlacksmithPayment { metal, arcane })
}

fn inventory_next(session: &SimpleSession) -> Option<(Command, Option<ItemType>)> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let toilet_unlocked = gs.character.level >= 100 && gs.tavern.toilet.is_some_and(|t| t.aura > 0);

    if toilet_unlocked {
        if let Some(toilet) = gs.tavern.toilet {
            if toilet.mana_total > 0 && toilet.mana_currently >= toilet.mana_total {
                return Some((Command::ToiletFlush, None));
            }
        }
    }

    let main_attr_pot = PotionType::from(gs.character.class.main_attribute());

    for (idx, active) in gs.character.active_potions.iter().enumerate() {
        if let Some(ap) = active {
            let is_main_attribute = ap.typ == main_attr_pot;
            let is_con = ap.typ == PotionType::Constitution;
            let is_wing = ap.typ == PotionType::EternalLife;

            if !is_main_attribute && !is_con && !is_wing {
                return Some((Command::RemovePotion { pos: idx }, Some(ItemType::Potion(*ap))));
            }
        }
    }

    let fs = crate::constant::INVENTORY_MIN_FREE_SLOTS;

    let can_sell = gs.character.inventory.count_free_slots() < fs;

    for (bag_pos, slot) in gs.character.inventory.iter() {
        let Some(item) = slot else {
            continue;
        };

        let (from_pos, item_ident) = (PlayerItemPosition::from(bag_pos), item.command_ident());

        if item.typ == ItemType::ToiletKey && !toilet_unlocked {
            return Some((Command::ToiletOpen, Some(item.typ.clone())));
        }

        if let ItemType::Potion(potion) = item.typ {
            let is_main_attr = potion.typ == main_attr_pot;

            let is_constitution = potion.typ == PotionType::Constitution;
            let is_winged_bottle = potion.typ == PotionType::EternalLife;

            if !is_main_attr && !is_constitution && !is_winged_bottle {
                continue;
            }

            let aps = gs.character.active_potions;

            let active_idx_and_pot = aps.iter().enumerate().find_map(|(idx, active)| {
                active.as_ref().filter(|a| a.typ == potion.typ).map(|a| (idx, a))
            });

            match active_idx_and_pot {
                Some((idx, ap)) => {
                    if ap.size < potion.size {
                        let cmd = Command::RemovePotion { pos: idx };

                        return Some((cmd, Some(ItemType::Potion(*ap))));
                    }

                    if ap.size > potion.size {
                        continue;
                    }

                    let (ts, max) = (chrono::Local::now().timestamp(), 12 * 24 * 60 * 60);

                    if !ap.expires.map_or(false, |exp| exp.timestamp() - ts >= max) {
                        let from = ItemPosition::from(from_pos);

                        let cmd = Command::UsePotion { from, item_ident };

                        return Some((cmd, Some(item.typ.clone())));
                    }
                }

                None => {
                    let cmd = Command::UsePotion { from: ItemPosition::from(from_pos), item_ident };

                    return Some((cmd, Some(item.typ.clone())));
                }
            }
        }

        if let ItemType::Gem(gem) = &item.typ {
            let player_attr = gs.character.class.main_attribute();
            let match_attrib = matches_attr(gem.typ, player_attr);

            let is_elig = gem.typ == GemType::All || gem.typ == GemType::Legendary || match_attrib;

            if is_elig {
                let (mut best_target, mut min_filled_value) = (None, u32::MAX);

                for slot in EquipmentSlot::iter() {
                    if let Some(eq_item) = gs.character.equipment.0[slot].as_ref() {
                        if let Some(gem_slot) = eq_item.gem_slot {
                            match gem_slot {
                                GemSlot::Empty => {
                                    let to_slot = slot;

                                    let cmd = Command::Equip { from_pos, to_slot, item_ident };

                                    return Some((cmd, Some(item.typ.clone())));
                                }

                                GemSlot::Filled(inserted_gem) => {
                                    let higher_value = gem.value > inserted_gem.value;

                                    if higher_value && inserted_gem.value < min_filled_value {
                                        min_filled_value = inserted_gem.value;

                                        best_target = Some(slot);
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(to_slot) = best_target {
                    let cmd = Command::Equip { from_pos, to_slot, item_ident };

                    return Some((cmd, Some(item.typ.clone())));
                }
            }

            if let Some(ref companions) = gs.dungeons.companions {
                let comps = [CompanionClass::Warrior, CompanionClass::Mage, CompanionClass::Scout];

                let make_cmd = |to_slot, to_companion| {
                    return Command::EquipCompanion { from_pos, to_slot, item_ident, to_companion };
                };

                for comp in comps {
                    let player_attr = Class::from(comp).main_attribute();
                    let matches_att = matches_attr(gem.typ, player_attr);

                    let (all, leg) = (gem.typ == GemType::All, gem.typ == GemType::Legendary);

                    let is_eligible = all || leg || matches_att;

                    if is_eligible {
                        let (mut best_target, mut min_filled_value) = (None, u32::MAX);

                        for slot in EquipmentSlot::iter() {
                            if let Some(eq_item) = companions[comp].equipment.0[slot].as_ref() {
                                if let Some(gem_slot) = eq_item.gem_slot {
                                    match gem_slot {
                                        GemSlot::Empty => {
                                            let cmd = make_cmd(slot, comp);

                                            return Some((cmd, Some(item.typ.clone())));
                                        }

                                        GemSlot::Filled(ins) => {
                                            let higher_value = gem.value > ins.value;

                                            if higher_value && ins.value < min_filled_value {
                                                min_filled_value = ins.value;

                                                best_target = Some(slot);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(to_slot) = best_target {
                            return Some((make_cmd(to_slot, comp), Some(item.typ.clone())));
                        }
                    }
                }
            }

            continue;
        }

        if let Some(to_slot) = target_equipment_slot(session, item) {
            let cmd = Command::Equip { from_pos, to_slot, item_ident };

            return Some((cmd, Some(item.typ.clone())));
        }

        let Some(slot) = item.typ.equipment_slot() else {
            continue;
        };

        for companion in [CompanionClass::Warrior, CompanionClass::Mage, CompanionClass::Scout] {
            if s_eq_comp(session, item, slot, companion) {
                let (to_slot, to_companion) = (slot, companion);

                let cmd = Command::EquipCompanion { from_pos, to_slot, item_ident, to_companion };

                return Some((cmd, Some(item.typ.clone())));
            }
        }
    }

    let mut surplus_items = Vec::new();

    for (bag_pos, slot) in gs.character.inventory.iter() {
        if let Some(item) = slot {
            if item.typ == ItemType::ToiletKey && !toilet_unlocked {
                continue;
            }

            surplus_items.push((bag_pos, item));
        }
    }

    if gs.character.level >= 90 && gs.blacksmith.is_some() {
        let blacksmith = gs.blacksmith.as_ref().unwrap();

        if blacksmith.dismantle_left > 0 {
            for &(bag_pos, item) in &surplus_items {
                if item.typ.equipment_slot().is_some() {
                    let reward = item.dismantle_reward();

                    if reward.arcane > crate::constant::BLACKSMITH_MIN_ARCANE_DISMANTLE {
                        let item_pos = PlayerItemPosition::from(bag_pos);

                        let (item_ident, a) = (item.command_ident(), BlacksmithAction::Dismantle);

                        let cmd = Command::Blacksmith { item_pos, action: a, item_ident };

                        return Some((cmd, Some(item.typ.clone())));
                    }
                }
            }
        }
    }

    if toilet_unlocked {
        if let Some(toilet) = gs.tavern.toilet {
            if toilet.sacrifices_left > 0 {
                let mut best_sacrifice = None;

                for &(bag_pos, item) in &surplus_items {
                    if item.is_epic() {
                        best_sacrifice = Some((bag_pos, item));

                        break;
                    }
                }

                if best_sacrifice.is_none() {
                    if let Some(&(bag_pos, item)) = surplus_items.first() {
                        best_sacrifice = Some((bag_pos, item));
                    }
                }

                if let Some((bag_pos, item)) = best_sacrifice {
                    let item_pos = PlayerItemPosition::from(bag_pos);

                    return Some((Command::ToiletDrop { item_pos }, Some(item.typ.clone())));
                }
            }
        }
    }

    if can_sell {
        if let Some(&(bag_pos, item)) = surplus_items.first() {
            let (item_pos, item_ident) = (PlayerItemPosition::from(bag_pos), item.command_ident());

            return Some((sell(session, item_pos, item_ident, item), Some(item.typ.clone())));
        }
    }

    if gs.character.level >= 90 && gs.blacksmith.is_some() {
        let bs = gs.blacksmith.as_ref().unwrap();

        let (mut tsm, mut tsa) = (0, 0);

        for slot in EquipmentSlot::iter() {
            if slot == EquipmentSlot::Shield && gs.character.class != Class::Assassin {
                continue;
            }

            if let Some(eq_item) = gs.character.equipment.0[slot].as_ref() {
                if let Some(costs) = socket_costs(eq_item) {
                    (tsm, tsa) = (tsm + costs.metal, tsa + costs.arcane);
                }
            }
        }

        for slot in EquipmentSlot::iter() {
            let item_pos = PlayerItemPosition::from(slot);

            if slot == EquipmentSlot::Shield && gs.character.class != Class::Assassin {
                continue;
            }

            let su = BlacksmithAction::SocketUpgrade;

            if let Some(eq_item) = gs.character.equipment.0[slot].as_ref() {
                if let Some(costs) = socket_costs(eq_item) {
                    if bs.metal >= costs.metal && bs.arcane >= costs.arcane {
                        let item_ident = eq_item.command_ident();

                        let cmd = Command::Blacksmith { item_pos, action: su, item_ident };

                        return Some((cmd, Some(eq_item.typ.clone())));
                    }
                }
            }
        }

        let mut slots_to_upgrade = vec![
            EquipmentSlot::Weapon,
            EquipmentSlot::BreastPlate,
            EquipmentSlot::FootWear,
            EquipmentSlot::Gloves,
            EquipmentSlot::Hat,
            EquipmentSlot::Belt,
            EquipmentSlot::Amulet,
            EquipmentSlot::Ring,
            EquipmentSlot::Talisman,
        ];

        if gs.character.class == Class::Assassin {
            slots_to_upgrade.push(EquipmentSlot::Shield);
        }

        for slot in slots_to_upgrade {
            let item_pos = PlayerItemPosition::from(slot);

            if let Some(eq_item) = gs.character.equipment.0[slot].as_ref() {
                if let Some(costs) = eq_item.upgrade_costs() {
                    let (em, ea) = (bs.metal >= costs.metal + tsm, bs.arcane >= costs.arcane + tsa);

                    if em && ea {
                        let cmd = Command::BlacksmithUpgradeItem { item_pos, amount: 1 };

                        return Some((cmd, Some(eq_item.typ.clone())));
                    }
                }
            }
        }
    }

    None
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2800.0, 1000.0, 1000.0, 6000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}

pub async fn inventory(session: &mut SimpleSession) {
    while let Some((cmd, item_typ)) = inventory_next(session) {
        let fmt_item = |t: &Option<ItemType>| {
            t.as_ref().map_or_else(|| "Unknown".to_string(), |i| format!("{:?}", i))
        };

        match &cmd {
            Command::Equip { .. } => {
                let message = format!("EQUIPPING '{}' ITEM", fmt_item(&item_typ));

                log(session, &message);
            }

            Command::EquipCompanion { to_companion, .. } => {
                let (i, c) = (fmt_item(&item_typ), to_companion);

                let message = format!("EQUIPPING '{}' TO '{:?}' COMPANION", i, c);

                log(session, &message);
            }

            Command::SellShop { .. } => {
                let message = format!("SELLING '{}' ITEM", fmt_item(&item_typ));

                log(session, &message);
            }

            Command::UsePotion { .. } => {
                let message = format!("DRINKING '{}' POTION", fmt_item(&item_typ));

                log(session, &message);
            }

            Command::RemovePotion { .. } => {
                let message = format!("REMOVING '{}' POTION", fmt_item(&item_typ));

                log(session, &message);
            }

            Command::ToiletFlush => {
                log(session, "FLUSHING TOILET");
            }

            Command::ToiletDrop { .. } => {
                let message = format!("THROWING '{}' INTO TOILET", fmt_item(&item_typ));

                log(session, &message);
            }

            Command::WitchDropCauldron { .. } => {
                let message = format!("THROWING '{}' INTO WITCH CAULDRON", fmt_item(&item_typ));

                log(session, &message);
            }

            Command::ToiletOpen => {
                log(session, "UNLOCKING TOILET WITH KEY");
            }

            Command::Blacksmith { action, .. } => {
                let message = format!("BLACKSMITH '{:?}' ON '{}'", action, fmt_item(&item_typ));

                log(session, &message);
            }

            Command::BlacksmithUpgradeItem { .. } => {
                let message = format!("BLACKSMITH UPGRADE ON '{}'", fmt_item(&item_typ));

                log(session, &message);
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("INVENTORY SEND COMMAND ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}
