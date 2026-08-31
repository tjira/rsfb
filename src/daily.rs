use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::{Command, FortunePayment},
    gamestate::unlockables::HellevatorStatus,
    session::SimpleSession,
};

use crate::log::log;

fn daily_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let now = chrono::Local::now();

    if let Some(next) = gs.specials.calendar.next_possible {
        if now >= next {
            return Some(Command::CollectCalendar);
        }
    }

    if gs.specials.advent_calendar.is_some() {
        return Some(Command::CollectAdventsCalendar);
    }

    for i in 0..3 {
        let chest = &gs.specials.tasks.daily.rewards[i];
        if chest.required_points > 0 && gs.specials.tasks.daily.can_open_chest(i) {
            return Some(Command::CollectDailyQuestReward { pos: i });
        }
    }

    for i in 0..3 {
        let chest = &gs.specials.tasks.event.rewards[i];
        if chest.required_points > 0 && gs.specials.tasks.event.can_open_chest(i) {
            return Some(Command::CollectEventTaskReward { pos: i });
        }
    }

    if gs.character.level >= 10 {
        match gs.hellevator.status() {
            HellevatorStatus::Active(hellevator) => {
                if hellevator.rewards_yesterday.as_ref().is_some_and(|r| r.claimable()) {
                    return Some(Command::HellevatorClaimDailyYesterday);
                }

                if hellevator.rewards_today.as_ref().is_some_and(|r| r.claimable()) {
                    return Some(Command::HellevatorClaimDaily);
                }
            }
            _ => {}
        }
    }

    if let Some(next) = gs.specials.wheel.next_free_spin {
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
                log(session, &format!("COLLECTING DAILY TASK CHEST NO. {}", pos + 1));
            }

            Command::CollectEventTaskReward { pos } => {
                log(session, &format!("COLLECTING EVENT TASK CHEST NO. {}", pos + 1));
            }

            Command::SpinWheelOfFortune { payment: FortunePayment::FreeTurn } => {
                log(session, "SPINNING WHEEL OF FORTUNE (FREE SPIN)");
            }

            Command::HellevatorClaimDaily => {
                log(session, "COLLECTING HELLEVATOR DAILY CHESTS");
            }

            Command::HellevatorClaimDailyYesterday => {
                log(session, "COLLECTING HELLEVATOR YESTERDAY'S DAILY CHESTS");
            }

            Command::HellevatorClaimFinal => {
                log(session, "COLLECTING HELLEVATOR FINAL REWARD");
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            let message = format!("DAILY SEND COMMAND ERROR: {:?}", err);

            log(session, &message);

            let _ = session.send_command(Command::Update).await;

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
