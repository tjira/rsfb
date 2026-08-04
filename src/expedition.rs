use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    gamestate::tavern::{AvailableTasks, CurrentAction, ExpeditionStage},
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
                log(session, "FIGHTING BOSS IN EXPEDITION");

                return Some(Command::ExpeditionContinue);
            }

            ExpeditionStage::Rewards(_) => {
                log(session, "PICKING REWARD IN EXPEDITION");

                return Some(Command::ExpeditionPickReward { pos: 0 });
            }

            ExpeditionStage::Encounters(encounters) if !encounters.is_empty() => {
                log(session, "PICKING ENCOUNTER IN EXPEDITION");

                return Some(Command::ExpeditionPickEncounter { pos: 0 });
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
        CurrentAction::Expedition => {
            log(session, "FINISHING EXPEDITION");

            return Some(Command::ExpeditionContinue);
        }

        CurrentAction::Idle => {
            log(session, "READY TO START NEW EXPEDITION");
        }

        _ => return None,
    }

    let thirst = gs.tavern.thirst_for_adventure_sec;

    let AvailableTasks::Expeditions(tasks) = gs.tavern.available_tasks() else {
        log(session, "NO EXPEDITIONS AVAILABLE");

        return None;
    };

    let Some(task) = tasks.first() else {
        log(session, "EXPEDITION LIST IS EMPTY");

        return None;
    };

    let cost = task.thirst_for_adventure_sec;

    if cost <= thirst {
        let message = format!("STARTING EXPEDITION FOR {} THIRST FOR ADVENTURE", cost / 60);

        log(session, &message);

        return Some(Command::ExpeditionStart { pos: 0 });
    }

    log(session, "NOT ENOUGH THIRST FOR ADVENTURE");

    None
}

pub async fn expedition(session: &mut SimpleSession) {
    while let Some(cmd) = expedition_next(session) {
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
    let (mean, std, min, max): (f64, f64, f64, f64) = (2000.0, 1000.0, 500.0, 3500.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
