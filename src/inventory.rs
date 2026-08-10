use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use strum::IntoEnumIterator;

use sf_api::{
    command::{AttributeType, BlacksmithAction, Command},
    gamestate::character::Class,
    gamestate::dungeons::CompanionClass,
    gamestate::items::{EquipmentSlot, GemSlot, GemType, Item},
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

pub(crate) fn is_equippable(session: &SimpleSession, item: &Item) -> bool {
    let Some(slot) = item.typ.equipment_slot() else {
        return false;
    };

    if should_equip(session, item, slot) {
        return true;
    }

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

    let toilet_unlocked =
        gs.character.level >= 100 && gs.tavern.toilet.map_or(false, |t| t.aura > 0);

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

fn inventory_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let toilet_unlocked =
        gs.character.level >= 100 && gs.tavern.toilet.map_or(false, |t| t.aura > 0);

    if toilet_unlocked {
        if let Some(toilet) = gs.tavern.toilet {
            if toilet.mana_currently >= toilet.mana_total {
                return Some(Command::ToiletFlush);
            }
        }
    }

    let can_sell = gs.character.inventory.count_free_slots() < 3;

    for (bag_pos, slot) in gs.character.inventory.iter() {
        let Some(item) = slot else {
            continue;
        };

        let (from_pos, item_ident) = (PlayerItemPosition::from(bag_pos), item.command_ident());

        if item.typ == ItemType::ToiletKey && !toilet_unlocked {
            return Some(Command::ToiletOpen);
        }

        if let ItemType::Potion(potion) = item.typ {
            let is_main_attr = potion.typ == PotionType::from(gs.character.class.main_attribute());

            let is_constitution = potion.typ == PotionType::Constitution;
            let is_winged_bottle = potion.typ == PotionType::EternalLife;

            if !is_main_attr && !is_constitution && !is_winged_bottle {
                if can_sell {
                    return Some(sell(session, from_pos, item_ident, item));
                }

                continue;
            }

            let aps = gs.character.active_potions;

            let active_idx_and_pot = aps.iter().enumerate().find_map(|(idx, active)| {
                active.as_ref().filter(|a| a.typ == potion.typ).map(|a| (idx, a))
            });

            match active_idx_and_pot {
                Some((idx, ap)) => {
                    if ap.size < potion.size {
                        return Some(Command::RemovePotion { pos: idx });
                    }

                    if ap.size > potion.size {
                        if can_sell {
                            return Some(sell(session, from_pos, item_ident, item));
                        }

                        continue;
                    }

                    let (ts, max) = (chrono::Local::now().timestamp(), 12 * 24 * 60 * 60);

                    if !ap.expires.map_or(false, |exp| exp.timestamp() - ts >= max) {
                        let from = ItemPosition::from(from_pos);

                        return Some(Command::UsePotion { from, item_ident });
                    }
                }

                None => {
                    return Some(Command::UsePotion {
                        from: ItemPosition::from(from_pos),
                        item_ident,
                    });
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

                                    return Some(Command::Equip { from_pos, to_slot, item_ident });
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
                    return Some(Command::Equip { from_pos, to_slot, item_ident });
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
                                            return Some(make_cmd(slot, comp));
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
                            return Some(make_cmd(to_slot, comp));
                        }
                    }
                }
            }

            if can_sell {
                return Some(sell(session, from_pos, item_ident, item));
            }

            continue;
        }

        let Some(slot) = item.typ.equipment_slot() else {
            continue;
        };

        if should_equip(session, item, slot) {
            return Some(Command::Equip { from_pos, to_slot: slot, item_ident });
        }

        for companion in [CompanionClass::Warrior, CompanionClass::Mage, CompanionClass::Scout] {
            if s_eq_comp(session, item, slot, companion) {
                let (to_slot, to_companion) = (slot, companion);

                let cmd = Command::EquipCompanion { from_pos, to_slot, item_ident, to_companion };

                return Some(cmd);
            }
        }

        if can_sell {
            return Some(sell(session, from_pos, item_ident, item));
        }

        continue;
    }

    if gs.character.level >= 90 && gs.blacksmith.is_some() {
        let bs = gs.blacksmith.as_ref().unwrap();

        for slot in EquipmentSlot::iter() {
            let item_pos = PlayerItemPosition::from(slot);

            if slot == EquipmentSlot::Shield {
                continue;
            }

            let su = BlacksmithAction::SocketUpgrade;

            if let Some(eq_item) = gs.character.equipment.0[slot].as_ref() {
                if eq_item.gem_slot.is_none() {
                    let level = eq_item.item_quality as u64;

                    let (metal_cost, arcane_cost) = (level * 10, (level * 5 / 10) * 10);

                    if bs.metal >= metal_cost && bs.arcane >= arcane_cost {
                        let item_ident = eq_item.command_ident();

                        return Some(Command::Blacksmith { item_pos, action: su, item_ident });
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
    while let Some(cmd) = inventory_next(session) {
        match &cmd {
            Command::Equip { to_slot, .. } => {
                let message = format!("EQUIPPING ITEM TO '{:?}' SLOT", to_slot);

                log(session, &message);
            }

            Command::EquipCompanion { to_slot, to_companion, .. } => {
                let message = format!("EQUIPPING '{:?}' TO '{:?}' SLOT", to_companion, to_slot);

                log(session, &message);
            }

            Command::SellShop { .. } => {
                log(session, "SELLING WEAKER/INCOMPATIBLE ITEM");
            }

            Command::UsePotion { .. } => {
                log(session, "DRINKING POTION");
            }

            Command::ToiletFlush => {
                log(session, "FLUSHING TOILET");
            }

            Command::ToiletDrop { .. } => {
                log(session, "THROWING ITEM INTO TOILET");
            }

            Command::WitchDropCauldron { .. } => {
                log(session, "THROWING ITEM INTO WITCH CAULDRON");
            }

            Command::ToiletOpen => {
                log(session, "UNLOCKING TOILET WITH KEY");
            }

            Command::Blacksmith { action, item_pos, .. } => {
                let message = format!("BLACKSMITH '{:?}' ON {} POSITION", action, item_pos);

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
