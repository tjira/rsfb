use std::env;
use std::process;
use std::time::Duration;

use chrono::Timelike;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{command::Command, gamestate::tavern::CurrentAction, session::SimpleSession};

mod constant;
mod expedition;
mod inventory;
mod log;

use expedition::expedition;
use inventory::inventory;

#[tokio::main]
async fn main() -> Result<(), sf_api::error::SFError> {
    let username = env::var("USERNAME").unwrap_or_else(|_| {
        eprintln!("THE 'USERNAME' ENVIRONMENT VARIABLE IS MISSING");

        process::exit(1);
    });

    let password = env::var("PASSWORD").unwrap_or_else(|_| {
        eprintln!("THE 'PASSWORD' ENVIRONMENT VARIABLE IS MISSING");

        process::exit(1);
    });

    let sessions = match SimpleSession::login_sf_account(&username, &password).await {
        Ok(sessions) => {
            println!("ACCOUNT LOGGED IN");

            sessions
        }

        Err(_) => {
            eprintln!("FAILED TO LOGIN");

            process::exit(1);
        }
    };

    let mut handles = Vec::new();

    for session in sessions {
        handles.push(tokio::spawn(process_session(session)));
    }

    for handle in handles {
        if let Err(err) = handle.await {
            eprintln!("TASK JOIN ERROR: {:?}", err);
        }
    }

    Ok(())
}

async fn process_session(mut session: SimpleSession) {
    if let Err(err) = session.send_command(Command::Update).await {
        return eprintln!("ERROR UPDATING SESSION ({:?})", err);
    }

    log::log(&session, "DOWNLOADED AND READY TO RUN");

    loop {
        let hour = chrono::Local::now().hour();

        if session.game_state().is_none() {
            return eprintln!("GAME STATE IS NOT POPULATED");
        }

        let thirst = session.game_state().unwrap().tavern.thirst_for_adventure_sec;

        inventory(&mut session).await;

        let tavern_action = session.game_state().unwrap().tavern.current_action;

        if session.game_state().unwrap().character.inventory.count_free_slots() == 0 {
            log::log(&session, "FULL INVENTORY, SKIPPING EXPEDITIONS");

            wait_between_actions().await;

            continue;
        }

        let expedition_running = tavern_action == CurrentAction::Expedition;

        if (thirst > 0 || expedition_running) && hour > constant::EXPEDITION_START_HOUR {
            expedition(&mut session).await;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (10000.0, 1000.0, 5000.0, 15000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
