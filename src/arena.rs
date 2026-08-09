use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use strum::IntoEnumIterator;

use sf_api::{
    command::{Command, IdleUpgradeAmount},
    gamestate::idle::IdleBuildingType,
    session::SimpleSession,
};

use crate::log::log;

fn arena_next(session: &SimpleSession, vi: &std::collections::HashSet<u32>) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    if gs.arena.fights_for_xp >= 10 {
        return None;
    }

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

        if vi.contains(&id) {
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

pub async fn arena_manager(session: &mut SimpleSession) {
    if let Some(gs) = session.game_state() {
        if let Some(idle_game) = &gs.idle_game {
            let mut required_runes = &idle_game.current_runes * 2;

            if idle_game.current_runes == 0.into() {
                required_runes = 2.into();
            }

            if idle_game.sacrifice_runes >= required_runes {
                log(session, &format!("SACRIFICING IN ARENA MANAGER"));

                if let Err(err) = session.send_command(Command::IdleSacrifice).await {
                    log(session, &format!("ARENA MANAGER SACRIFICE ERROR ({:?})", err));
                }

                wait_between_actions().await;
            }
        }
    }

    loop {
        let Some(gs) = session.game_state() else {
            break;
        };

        let Some(idle_game) = &gs.idle_game else {
            break;
        };

        let mut cheapest = None;

        for building_type in IdleBuildingType::iter() {
            let building = &idle_game.buildings[building_type];

            let (mut cost, mut amount) = (building.upgrade_cost.clone(), IdleUpgradeAmount::One);

            if building.level >= 10 {
                (cost, amount) = (building.upgrade_cost_10x.clone(), IdleUpgradeAmount::Ten);
            }

            if cheapest.is_none() {
                cheapest = Some((building_type, amount, cost.clone()));
            }

            if let Some((_, _, ref cheapest_cost)) = cheapest {
                if cost < *cheapest_cost {
                    cheapest = Some((building_type, amount, cost));
                }
            }
        }

        let Some((building_type, amount, cost)) = cheapest else {
            break;
        };

        if cost > idle_game.current_money {
            break;
        }

        let msg = format!("IDLE UPGRADE '{:?}' BUILDING", building_type);

        log(session, &msg);

        let cmd = Command::IdleUpgrade { typ: building_type, amount };

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("ARENA MANAGER UPGRADE ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}

pub async fn arena(session: &mut SimpleSession) {
    arena_manager(session).await;

    if let Some(gs) = session.game_state() {
        if gs.arena.fights_for_xp >= 10 {
            return;
        }

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

    let mut viewed_ids = std::collections::HashSet::new();

    while let Some(cmd) = arena_next(session, &viewed_ids) {
        match &cmd {
            Command::ViewPlayer { ident } => {
                if let Ok(id) = ident.parse::<u32>() {
                    viewed_ids.insert(id);
                }
            }

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
