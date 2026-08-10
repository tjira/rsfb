use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use rand::{Rng, thread_rng};
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    gamestate::items::{Enchantment, EquipmentSlot},
    gamestate::rewards::{Event, RewardType},
    gamestate::tavern::{AvailableTasks, CurrentAction, ExpeditionSpecial, ExpeditionStage},
    session::SimpleSession,
};

use crate::log::log;

static LAST_LOGGED_WAITS: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

fn expedition_next(session: &mut SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    if let Some(stage) = gs.tavern.expeditions.active().map(|a| a.current_stage()) {
        match stage {
            ExpeditionStage::Boss(_) => {
                return Some(Command::ExpeditionContinue);
            }

            ExpeditionStage::Rewards(rewards) => {
                let mut pos = None;

                for (i, r) in rewards.iter().enumerate() {
                    if r.typ == RewardType::Mushrooms {
                        pos = Some(i);

                        break;
                    }
                }

                if pos.is_none() {
                    for (i, r) in rewards.iter().enumerate() {
                        if matches!(r.typ, RewardType::Fruit(_) | RewardType::FruitBasket) {
                            pos = Some(i);

                            break;
                        }
                    }
                }

                return Some(Command::ExpeditionPickReward { pos: pos.unwrap_or(0) });
            }

            ExpeditionStage::Encounters(encs) if !encs.is_empty() => {
                let pos = rand::thread_rng().gen_range(0..encs.len());

                return Some(Command::ExpeditionPickEncounter { pos });
            }

            ExpeditionStage::Waiting { busy_until, .. } => {
                let timestamp = busy_until.timestamp();

                if (timestamp - get_last_wait(session.username())).abs() > 15 {
                    get_waits_map().insert(session.username().to_string(), timestamp);

                    let remaining = (busy_until - chrono::Local::now()).num_seconds().max(0);

                    let mins = remaining / 60;
                    let secs = remaining % 60;

                    let tf = busy_until.format("%H:%M:%S");

                    let message = format!("EXPEDITION WAITING {mins}:{secs:02} UNTIL {tf}");

                    log(session, &message);
                }

                return None;
            }

            _ => return None,
        }
    }

    match gs.tavern.current_action {
        CurrentAction::Expedition => return Some(Command::ExpeditionContinue),

        CurrentAction::Idle => {}

        _ => return None,
    }

    if can_drink_beer(session) {
        return Some(Command::BuyBeer);
    }

    let thirst = gs.tavern.thirst_for_adventure_sec;

    let AvailableTasks::Expeditions(tasks) = gs.tavern.available_tasks() else {
        log(session, "NO EXPEDITIONS AVAILABLE");

        return None;
    };

    let mut pos = 0;

    for (i, task) in tasks.iter().enumerate() {
        if task.special == Some(ExpeditionSpecial::Egg) {
            pos = i;

            break;
        }

        if task.special == Some(ExpeditionSpecial::DailyTask) {
            pos = i;
        }
    }

    let Some(task) = tasks.get(pos) else {
        log(session, "EXPEDITION LIST IS EMPTY");

        return None;
    };

    let cost = task.thirst_for_adventure_sec;

    if cost <= thirst {
        let message = format!("STARTING EXPEDITION FOR {} THIRST FOR ADVENTURE", cost / 60);

        log(session, &message);

        return Some(Command::ExpeditionStart { pos });
    }

    log(session, "NOT ENOUGH THIRST FOR ADVENTURE");

    None
}

pub async fn expedition(session: &mut SimpleSession) {
    while let Some(cmd) = expedition_next(session) {
        match &cmd {
            Command::BuyBeer => {
                log(session, "DRINKING A FREE BEER");
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            let message = format!("EXPEDITION SEND COMMAND ERROR: {:?}", err);

            log(session, &message);

            break;
        }

        wait_between_actions().await;
    }
}

fn get_waits_map() -> std::sync::MutexGuard<'static, HashMap<String, i64>> {
    LAST_LOGGED_WAITS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap()
}

fn get_last_wait(username: &str) -> i64 {
    get_waits_map().get(username).copied().unwrap_or(0)
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (3500.0, 1500.0, 1200.0, 8000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}

pub fn can_drink_beer(session: &SimpleSession) -> bool {
    let Some(gs) = session.game_state() else {
        return false;
    };

    let ca = gs.tavern.current_action;

    if ca == CurrentAction::Idle && gs.tavern.beer_drunk < gs.tavern.beer_max {
        let event_free = gs.specials.events.active.contains(&Event::OneBeerTwoBeerFreeBeer);

        let mut belt_free = false;

        if gs.tavern.beer_drunk == 0 || gs.tavern.beer_drunk == 10 {
            let belt = gs.character.equipment.0[EquipmentSlot::Belt].as_ref();

            belt_free = belt.map_or(false, |i| i.enchantment == Some(Enchantment::ThirstyWanderer));
        }

        if event_free || belt_free {
            let max_thirst = 6000 + (gs.tavern.beer_max as u32 * 1200);

            if gs.tavern.thirst_for_adventure_sec + 1200 <= max_thirst {
                return true;
            }
        }
    }

    false
}
