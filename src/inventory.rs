use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    gamestate::GameState,
    gamestate::items::{EquipmentSlot, Item, PlayerItemPosition},
    session::SimpleSession,
};

use crate::log::log;

fn should_equip(item: &Item, gs: &GameState, slot: EquipmentSlot) -> bool {
    let attr = gs.character.class.main_attribute();

    if !item.can_be_equipped_by(gs.character.class) {
        return false;
    }

    let equipped = gs.character.equipment.0[slot].as_ref();

    item.attributes[attr] > equipped.map(|eq| eq.attributes[attr]).unwrap_or(0)
}

fn inventory_next(gs: &GameState) -> Option<Command> {
    for (bag_pos, slot) in gs.character.inventory.iter() {
        let Some(item) = slot else {
            continue;
        };

        let Some(slot) = item.typ.equipment_slot() else {
            continue;
        };

        let (from_pos, item_ident) = (PlayerItemPosition::from(bag_pos), item.command_ident());

        if should_equip(item, gs, slot) {
            return Some(Command::Equip { from_pos, to_slot: slot, item_ident });
        }

        if !should_equip(item, gs, slot) {
            return Some(Command::SellShop { item_pos: from_pos, item_ident });
        }
    }

    None
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2000.0, 1000.0, 500.0, 3500.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}

pub async fn inventory(session: &mut SimpleSession) {
    while let Some(gs) = session.game_state() {
        let Some(cmd) = inventory_next(gs) else {
            break;
        };

        match &cmd {
            Command::Equip { to_slot, .. } => {
                let message = format!("EQUIPPING ITEM TO '{:?}' SLOT", to_slot);

                log(session, &message);
            }

            Command::SellShop { .. } => {
                log(session, "SELLING WEAKER/INCOMPATIBLE ITEM");
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
