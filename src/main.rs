use std::env;
use std::process;

use sf_api::command::Command;
use sf_api::session::SimpleSession;

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

    let mut sessions = match SimpleSession::login_sf_account(&username, &password).await {
        Ok(sessions) => {
            println!("YOUR '{username}' ACCOUNT IS LOGGED IN");

            sessions
        }

        Err(_) => {
            eprintln!("FAILED TO LOGIN");

            process::exit(1);
        }
    };

    for session in &mut sessions {
        session.send_command(Command::Update).await.unwrap();

        log::log(session, "DOWNLOADED AND READY TO RUN")?;
    }

    for mut session in sessions {
        let gs = session.game_state().unwrap();

        if gs.character.inventory.count_free_slots() < 2 {
            log::log(&session, "LESS THAN 2 FREE INVENTORY SLOTS, SKIPPING EXPEDITION")?;

            continue;
        }

        if let Err(err) = expedition(&mut session).await {
            log::log(&session, &format!("ERROR RUNNING EXPEDITION ({:?})", err))?;
        }
    }

    Ok(())
}
