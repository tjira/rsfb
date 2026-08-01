use std::env;
use std::process;

use chrono::Timelike;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::command::Command;
use sf_api::session::SimpleSession;

mod constant;
mod expedition;
mod log;

use expedition::expedition;

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

    if let Err(err) = log::log(&session, "DOWNLOADED AND READY TO RUN") {
        return eprintln!("LOG ERROR ({:?})", err);
    }

    loop {
        let Some(gs) = session.game_state() else {
            return eprintln!("GAME STATE IS NOT POPULATED"); 
        };

        let hour = chrono::Local::now().hour();

        if gs.character.inventory.count_free_slots() == 0 {
            if let Err(err) = log::log(&session, "FULL INVENTORY, SKIPPING EXPEDITIONS") {
                return eprintln!("LOG ERROR ({:?})", err);
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            continue;
        }

        if gs.tavern.thirst_for_adventure_sec > 0 && hour > constant::EXPEDITION_START_HOUR {
            if let Err(err) = expedition(&mut session).await {
                let _ = log::log(&session, &format!("ERROR RUNNING EXPEDITION ({:?})", err));
            }
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (10000.0, 1000.0, 5000.0, 15000.0);

    let wait_time = Normal::new(mean, std).unwrap().sample(&mut thread_rng()).clamp(min, max) as u64;

    tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
}
