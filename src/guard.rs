use std::time::Duration;

use chrono::{Duration as ChronoDuration, Local, Timelike};
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{command::Command, gamestate::tavern::CurrentAction, session::SimpleSession};

use crate::log::log;

fn guard_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    match gs.tavern.current_action {
        CurrentAction::CityGuard { hours: _, busy_until } => {
            let now = Local::now();

            if now >= busy_until {
                return Some(Command::FinishWork);
            }

            None
        }

        CurrentAction::Idle => {
            let thirst = gs.tavern.thirst_for_adventure_sec;

            if thirst == 0 && !crate::expedition::can_drink_beer(session) {
                let (now, start) = (Local::now(), crate::constant::EXPEDITION_START_HOUR);

                if let Some(mut target) = now.date_naive().and_hms_opt(start, 0, 0) {
                    if now.hour() >= start {
                        target = target + ChronoDuration::days(1);
                    }

                    let hours_to_work = ((target - now.naive_local()).num_seconds() / 3600).min(10);

                    if hours_to_work >= 1 {
                        return Some(Command::StartWork { hours: hours_to_work as u8 });
                    }

                    return None;
                }

                return None;
            }

            return None;
        }

        _ => None,
    }
}

pub async fn guard(session: &mut SimpleSession) {
    while let Some(cmd) = guard_next(session) {
        match &cmd {
            Command::FinishWork => {
                log(session, "COLLECTING PAY FROM CITY GUARD");
            }

            Command::StartWork { hours } => {
                log(session, &format!("STARTING CITY GUARD FOR {hours} HOURS"));
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("CITY GUARD SEND COMMAND ERROR: {:?}", err));

            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (3000.0, 1000.0, 1200.0, 6500.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
