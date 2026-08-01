use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    error::SFError,
    gamestate::GameState,
    gamestate::items::{Item, EquipmentSlot, PlayerItemPosition},
    session::SimpleSession,
};

use crate::log::log;

fn should_equip(item: &Item, gs: &GameState, slot: EquipmentSlot) -> bool {
    if !item.can_be_equipped_by(gs.character.class) {
        return false;
    }

    let (main_attr, equipped_item) = (gs.character.class.main_attribute(), gs.character.equipment.0[slot].as_ref());

    item.attributes[main_attr] > equipped_item.map(|eq| eq.attributes[main_attr]).unwrap_or(0)
}

fn inventory_next(gs: &GameState) -> Option<Command> {
    for (bag_pos, slot) in gs.character.inventory.iter() {
        let Some(item) = slot else {
            continue;
        };

        let Some(eq_slot) = item.typ.equipment_slot() else {
            continue;
        };

        let (from_pos, item_ident) = (PlayerItemPosition::from(bag_pos), item.command_ident());

        if should_equip(item, gs, eq_slot) {
            return Some(Command::Equip { from_pos, to_slot: eq_slot, item_ident });
        }

        else {
            return Some(Command::SellShop { item_pos: from_pos, item_ident });
        }
    }

    None
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2000.0, 1000.0, 500.0, 3500.0);

    let wait_time = Normal::new(mean, std).unwrap().sample(&mut thread_rng()).clamp(min, max) as u64;

    tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
}

pub async fn inventory(session: &mut SimpleSession) -> Result<(), SFError> {
    while let Some(gs) = session.game_state() {
        let Some(cmd) = inventory_next(gs) else {
            break;
        };

        match &cmd {
            Command::Equip { to_slot, .. } => {
                log(session, &format!("EQUIPPING BETTER ITEM TO SLOT {:?}", to_slot))?;
            }

            Command::SellShop { .. } => {
                log(session, "SELLING WEAKER/INCOMPATIBLE ITEM")?;
            }

            _ => {}
        }

        session.send_command(cmd).await?;

        wait_between_actions().await;
    }

    Ok(())
}
