use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use strum::IntoEnumIterator;

use sf_api::{
    command::{Command, ShopType},
    gamestate::items::{ItemType, PlayerItemPosition, PotionType},
    session::SimpleSession,
};

use crate::log::log;

fn shop_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let main_attr = gs.character.class.main_attribute();

    let Some(free_slot) = gs.character.inventory.free_slot() else {
        return None;
    };

    let new_pos = PlayerItemPosition::from(free_slot);

    for shop_type in ShopType::iter() {
        let shop = &gs.shops[shop_type];

        for (shop_pos, item) in shop.iter() {
            if gs.character.silver >= item.price as u64 && item.mushroom_price == 0 {
                if let ItemType::Potion(potion) = &item.typ {
                    let aps = gs.character.active_potions;

                    let is_main_attr = potion.typ == PotionType::from(main_attr);
                    let is_constitution = potion.typ == PotionType::Constitution;

                    let is_health = potion.typ == PotionType::EternalLife;

                    if is_main_attr || is_constitution || is_health {
                        let active_pot = aps.iter().flatten().find(|a| a.typ == potion.typ);

                        let is_full = active_pot.map_or(false, |active| {
                            let ts = chrono::Local::now().timestamp();

                            let max = 12 * 24 * 60 * 60;

                            active.expires.map_or(false, |exp| exp.timestamp() - ts >= max)
                        });

                        let should_buy = match active_pot {
                            Some(active) => {
                                let c2 = active.size == potion.size && !is_full;

                                active.size < potion.size || c2
                            }
                            None => true,
                        };

                        if should_buy {
                            let item_ident = item.command_ident();

                            return Some(Command::BuyShop { shop_pos, new_pos, item_ident });
                        }
                    }
                }

                if item.typ == ItemType::QuickSandGlass {
                    let item_ident = item.command_ident();

                    return Some(Command::BuyShop { shop_pos, new_pos, item_ident });
                }

                if crate::inventory::is_equippable(session, item) {
                    let item_ident = item.command_ident();

                    return Some(Command::BuyShop { shop_pos, new_pos, item_ident });
                }
            }
        }
    }

    None
}

pub async fn shop(session: &mut SimpleSession) {
    let Some(gs) = session.game_state() else {
        return;
    };

    if gs.character.inventory.count_free_slots() == 0 {
        return;
    }

    while let Some(cmd) = shop_next(session) {
        match &cmd {
            Command::BuyShop { shop_pos, .. } => {
                log(session, &format!("BUYING FROM '{:?}' SHOP", shop_pos.typ));
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            let message = format!("SHOP SEND COMMAND ERROR: {:?}", err);

            log(session, &message);

            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2800.0, 1000.0, 1000.0, 6000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
