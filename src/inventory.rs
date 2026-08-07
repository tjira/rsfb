use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    gamestate::character::Class,
    gamestate::dungeons::CompanionClass,
    gamestate::items::{EquipmentSlot, Item, ItemCommandIdent, ItemPosition},
    gamestate::items::{ItemType, PlayerItemPosition, PotionType},
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

fn sell(s: &SimpleSession, pos: PlayerItemPosition, ii: ItemCommandIdent, item: &Item) -> Command {
    let Some(gs) = s.game_state() else {
        return Command::SellShop { item_pos: pos, item_ident: ii };
    };

    if let Some(toilet) = gs.tavern.toilet {
        if toilet.sacrifices_left > 0 {
            return Command::ToiletDrop { item_pos: pos };
        }
    }

    if let Some(witch) = &gs.witch {
        if let Some(slot) = item.typ.equipment_slot() {
            if witch.required_item == Some(slot) {
                return Command::WitchDropCauldron { item_pos: pos };
            }
        }
    }

    if gs.tavern.toilet.is_some() && !item.is_washed && item.typ.equipment_slot().is_some() {
        return Command::ToiletDrop { item_pos: pos };
    }

    Command::SellShop { item_pos: pos, item_ident: ii }
}

fn inventory_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    if let Some(toilet) = gs.tavern.toilet {
        if toilet.mana_currently >= toilet.mana_total {
            return Some(Command::ToiletFlush);
        }
    }

    for (bag_pos, slot) in gs.character.inventory.iter() {
        let Some(item) = slot else {
            continue;
        };

        let (from_pos, item_ident) = (PlayerItemPosition::from(bag_pos), item.command_ident());

        if let ItemType::Potion(potion) = item.typ {
            let is_main_attr = potion.typ == PotionType::from(gs.character.class.main_attribute());

            let is_constitution = potion.typ == PotionType::Constitution;
            let is_winged_bottle = potion.typ == PotionType::EternalLife;

            if !is_main_attr && !is_constitution && !is_winged_bottle {
                return Some(sell(session, from_pos, item_ident, item));
            }

            let is_full = gs.character.active_potions.iter().flatten().any(|a| {
                let (ts, max) = (chrono::Local::now().timestamp(), 12 * 24 * 60 * 60);

                a.typ == potion.typ && a.expires.map_or(false, |exp| exp.timestamp() - ts >= max)
            });

            if (is_main_attr || is_constitution || is_winged_bottle) && !is_full {
                return Some(Command::UsePotion { from: ItemPosition::from(from_pos), item_ident });
            }
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

        return Some(sell(session, from_pos, item_ident, item));
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

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("INVENTORY SEND COMMAND ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}
