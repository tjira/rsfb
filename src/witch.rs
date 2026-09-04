use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use strum::IntoEnumIterator;

use sf_api::{
    command::Command,
    gamestate::{GameState, dungeons::CompanionClass, items::Enchantment},
    session::SimpleSession,
};

use crate::log::log;

fn get_next_enchant_action(gs: &GameState) -> Option<(Command, String)> {
    let witch = gs.witch.as_ref()?;

    for e in Enchantment::iter() {
        if let Some(id) = witch.enchantments[e] {
            let slot = e.equipment_slot();

            if let Some(item) = &gs.character.equipment.0[slot] {
                let cost = witch.enchantment_price;

                if item.enchantment != Some(e) && gs.character.silver >= cost {
                    let msg = format!("ENCHANTING '{e:?}' ON PLAYER");

                    return Some((Command::WitchEnchant { enchantment: id }, msg));
                }
            }

            if let Some(ref companions) = gs.dungeons.companions {
                let comps = [CompanionClass::Warrior, CompanionClass::Mage, CompanionClass::Scout];

                for companion in comps {
                    let comp = &companions[companion];

                    if comp.level == 0 {
                        continue;
                    }

                    if let Some(item) = &comp.equipment.0[slot] {
                        let cost = witch.enchantment_price;

                        if item.enchantment != Some(e) && gs.character.silver >= cost {
                            let msg = format!("ENCHANTING '{e:?}' ON '{companion:?}' COMPANION");

                            let c = Command::WitchEnchantCompanion { enchantment: id, companion };

                            return Some((c, msg));
                        }
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

pub async fn witch(session: &mut SimpleSession) {
    loop {
        let Some(gs) = session.game_state() else {
            break;
        };

        let witch_unlocked = gs.character.level >= 66 && gs.witch.is_some();

        if !witch_unlocked {
            break;
        }

        let Some((cmd, msg)) = get_next_enchant_action(gs) else {
            break;
        };

        log(session, &msg);

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("WITCH ENCHANT ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}
