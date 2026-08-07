use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use strum::IntoEnumIterator;

use sf_api::{
    command::Command,
    gamestate::fortress::{FortressBuildingType, FortressResourceType},
    misc::EnumMapGet,
    session::SimpleSession,
};

use crate::log::log;

fn fortress_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let Some(ref fortress) = gs.fortress else {
        return None;
    };

    if let Some(target) = fortress.building_upgrade.target {
        if let Some(finish) = fortress.building_upgrade.finish {
            if Local::now() >= finish {
                return Some(Command::FortressBuildFinish { f_type: target, mushrooms: 0 });
            }
        }
    }

    let is_collect_time = fortress.last_collectable_updated.map_or(true, |last_updated| {
        let (interval, window) = (chrono::Duration::minutes(30), chrono::Duration::minutes(2));

        let time_since_update = Local::now() - last_updated;

        time_since_update >= interval || time_since_update < window
    });

    if is_collect_time {
        let wood = fortress.resources.get(FortressResourceType::Wood);

        if wood.production.last_collectable > 0 && wood.current < wood.limit {
            return Some(Command::FortressGather { resource: FortressResourceType::Wood });
        }

        let stone = fortress.resources.get(FortressResourceType::Stone);

        if stone.production.last_collectable > 0 && stone.current < stone.limit {
            return Some(Command::FortressGather { resource: FortressResourceType::Stone });
        }

        let exp = fortress.resources.get(FortressResourceType::Experience);

        if exp.production.last_collectable > 0 {
            return Some(Command::FortressGather { resource: FortressResourceType::Experience });
        }
    }

    if fortress.building_upgrade.target.is_none() {
        let mut buildable = Vec::new();

        for building_type in FortressBuildingType::iter() {
            if fortress.can_build(building_type, gs.character.silver) {
                let building = fortress.buildings.get(building_type);

                if building.level < fortress.building_max_lvl as u16 {
                    buildable.push((building_type, building.level));
                }
            }
        }

        if let Some((best_building, _)) = buildable.into_iter().min_by_key(|&(_, lvl)| lvl) {
            return Some(Command::FortressBuild { f_type: best_building });
        }
    }

    None
}

pub async fn fortress(session: &mut SimpleSession) {
    if let Some(gs) = session.game_state() {
        if gs.character.level < 25 || gs.fortress.is_none() {
            return;
        }
    }

    while let Some(cmd) = fortress_next(session) {
        match &cmd {
            Command::FortressBuildFinish { f_type, .. } => {
                log(session, &format!("FINISHING '{:?}' BUILDING IN FORTRESS", f_type));
            }

            Command::FortressGather { resource } => {
                log(session, &format!("GATHERING '{:?}' FROM FORTRESS", resource));
            }

            Command::FortressBuild { f_type } => {
                log(session, &format!("STARTING BUILDING '{:?}' IN FORTRESS", f_type));
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("FORTRESS SEND COMMAND ERROR ({:?})", err));

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
