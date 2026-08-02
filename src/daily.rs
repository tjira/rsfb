use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::{Command, FortunePayment},
    session::SimpleSession,
};

use crate::log::log;

fn daily_next(session: &SimpleSession) -> Option<Command> {
    let now = chrono::Local::now();

    if let Some(next) = session.game_state().unwrap().specials.calendar.next_possible {
        if now >= next {
            return Some(Command::CollectCalendar);
        }
    }

    if session.game_state().unwrap().specials.advent_calendar.is_some() {
        return Some(Command::CollectAdventsCalendar);
    }

    for i in 0..3 {
        if session.game_state().unwrap().specials.tasks.daily.can_open_chest(i) {
            return Some(Command::CollectDailyQuestReward { pos: i });
        }
    }

    for i in 0..3 {
        if session.game_state().unwrap().specials.tasks.event.can_open_chest(i) {
            return Some(Command::CollectEventTaskReward { pos: i });
        }
    }

    if let Some(next) = session.game_state().unwrap().specials.wheel.next_free_spin {
        if now >= next {
            return Some(Command::SpinWheelOfFortune { payment: FortunePayment::FreeTurn });
        }
    }

    None
}

pub async fn daily(session: &mut SimpleSession) {
    while let Some(cmd) = daily_next(session) {
        match &cmd {
            Command::CollectCalendar => {
                log(session, "COLLECTING DAILY CALENDAR REWARD");
            }

            Command::CollectAdventsCalendar => {
                log(session, "COLLECTING ADVENT CALENDAR DOOR REWARD");
            }

            Command::CollectDailyQuestReward { pos } => {
                log(session, &format!("COLLECTING DAILY TASK CHEST No. {}", pos + 1));
            }

            Command::CollectEventTaskReward { pos } => {
                log(session, &format!("COLLECTING EVENT TASK CHEST No. {}", pos + 1));
            }

            Command::SpinWheelOfFortune { payment: FortunePayment::FreeTurn } => {
                log(session, "SPINNING WHEEL OF FORTUNE (FREE SPIN)");
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            let message = format!("DAILY SEND COMMAND ERROR: {:?}", err);

            log(session, &message);

            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2000.0, 1000.0, 500.0, 3500.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
