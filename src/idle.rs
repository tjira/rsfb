use strum::IntoEnumIterator;

use sf_api::{
    command::{Command, IdleUpgradeAmount},
    gamestate::idle::IdleBuildingType,
    session::SimpleSession,
};

use crate::log::log;

pub async fn idle(session: &mut SimpleSession) {
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

                crate::wait_between_actions(1000.0, 500.0, 200.0, 1800.0).await;
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

        crate::wait_between_actions(1000.0, 500.0, 200.0, 1800.0).await;
    }
}
