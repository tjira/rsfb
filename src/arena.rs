use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{command::Command, session::SimpleSession};

use crate::log::log;

fn arena_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    if let Some(next_fight) = gs.arena.next_free_fight {
        if Local::now() < next_fight + chrono::Duration::seconds(5) {
            return None;
        }
    }

    let mut missing_id = None;

    for &id in &gs.arena.enemy_ids {
        if id == 0 {
            continue;
        }

        if gs.lookup.lookup_pid(id).is_none() {
            missing_id = Some(id);

            break;
        }
    }

    if let Some(id) = missing_id {
        return Some(Command::ViewPlayer { ident: id.to_string() });
    }

    let mut opponents = Vec::new();

    for &id in &gs.arena.enemy_ids {
        if id == 0 {
            continue;
        }

        if let Some(opponent) = gs.lookup.lookup_pid(id) {
            opponents.push(opponent);
        }
    }

    if let Some(target) = opponents.into_iter().min_by_key(|o| o.level) {
        return Some(Command::Fight { name: target.name.clone(), use_mushroom: false });
    }

    None
}

pub async fn arena(session: &mut SimpleSession) {
    if let Some(gs) = session.game_state() {
        if let Some(next_fight) = gs.arena.next_free_fight {
            if Local::now() < next_fight + chrono::Duration::seconds(5) {
                return;
            }
        }
    }

    if let Err(err) = session.send_command(Command::CheckArena).await {
        log(session, &format!("FAILED TO UPDATE ARENA ({:?})", err));

        return;
    }

    while let Some(cmd) = arena_next(session) {
        match &cmd {
            Command::ViewPlayer { .. } => {}

            Command::Fight { name, .. } => {
                log(session, &format!("ATTACKING '{name}' IN ARENA"));
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("ARENA SEND COMMAND ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (3000.0, 1200.0, 1000.0, 7000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
