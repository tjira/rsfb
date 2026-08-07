use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use std::process;
use std::sync::Arc;
use std::time::Duration;

use chrono::Timelike;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use tokio::sync::Mutex;

use sf_api::{command::Command, gamestate::tavern::CurrentAction, session::SimpleSession};

mod arena;
mod constant;
mod daily;
mod dungeon;
mod expedition;
mod fortress;
mod guard;
mod guild;
mod inventory;
mod log;
mod skill;

use arena::arena;
use daily::daily;
use dungeon::dungeon;
use expedition::expedition;
use fortress::fortress;
use guard::guard;
use guild::guild;
use inventory::inventory;
use skill::skill;

#[derive(Debug, Clone)]
struct CharacterStatus {
    name: String,
    level: u16,
    class: String,
    gold: f64,
    mushrooms: u32,
    rank: u32,
    status: String,
}

type SharedStatusMap = Arc<Mutex<HashMap<String, CharacterStatus>>>;

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

    let (shared_map, num_sessions) = (Arc::new(Mutex::new(HashMap::new())), sessions.len());

    tokio::spawn({
        let shared_map = shared_map.clone();

        async move {
            while shared_map.lock().await.len() < num_sessions {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }

            print_character_table(&shared_map).await;

            let mut interval = tokio::time::interval(Duration::from_secs(300));

            interval.tick().await;

            loop {
                interval.tick().await;

                print_character_table(&shared_map).await;
            }
        }
    });

    let mut handles = Vec::new();

    for session in sessions {
        handles.push(tokio::spawn(process_session(session, shared_map.clone())));
    }

    for handle in handles {
        if let Err(err) = handle.await {
            eprintln!("TASK JOIN ERROR: {:?}", err);
        }
    }

    Ok(())
}

async fn update_character_status(session: &SimpleSession, shared_map: &SharedStatusMap) {
    if let Some(gs) = session.game_state() {
        let status = match &gs.tavern.current_action {
            CurrentAction::Idle => "IDLE".to_string(),
            CurrentAction::CityGuard { hours, .. } => format!("WORKING ({}H)", hours),
            CurrentAction::Quest { quest_idx, .. } => format!("QUEST {}", quest_idx + 1),
            CurrentAction::Expedition => "EXPEDITION".to_string(),
            CurrentAction::Unknown(_) => "UNKNOWN".to_string(),
        };

        let mut map = shared_map.lock().await;

        let status = CharacterStatus {
            name: gs.character.name.clone(),
            level: gs.character.level,
            class: log::get_class_name(gs.character.class).to_string(),
            gold: (gs.character.silver as f64) / 100.0,
            mushrooms: gs.character.mushrooms,
            rank: gs.character.rank,
            status: status,
        };

        map.insert(gs.character.name.clone(), status);
    }
}

fn format_character_table(map: &HashMap<String, CharacterStatus>) -> String {
    let mut buffer = String::new();

    const WIDTHS: (usize, usize, usize, usize, usize, usize, usize) = (20, 5, 13, 10, 9, 8, 25);

    let border = format!(
        "+{}+{}+{}+{}+{}+{}+{}+",
        "-".repeat(WIDTHS.0 + 2),
        "-".repeat(WIDTHS.1 + 2),
        "-".repeat(WIDTHS.2 + 2),
        "-".repeat(WIDTHS.3 + 2),
        "-".repeat(WIDTHS.4 + 2),
        "-".repeat(WIDTHS.5 + 2),
        "-".repeat(WIDTHS.6 + 2)
    );

    let header = ("CHARACTER NAME", "LEVEL", "CLASS", "GOLD", "MUSHROOMS", "RANK", "STATUS");

    let _ = writeln!(buffer, "{}", border);

    let _ = writeln!(
        buffer,
        "| {:<w0$} | {:<w1$} | {:<w2$} | {:<w3$} | {:<w4$} | {:<w5$} | {:<w6$} |",
        header.0,
        header.1,
        header.2,
        header.3,
        header.4,
        header.5,
        header.6,
        w0 = WIDTHS.0,
        w1 = WIDTHS.1,
        w2 = WIDTHS.2,
        w3 = WIDTHS.3,
        w4 = WIDTHS.4,
        w5 = WIDTHS.5,
        w6 = WIDTHS.6
    );

    let _ = writeln!(buffer, "{}", border);

    let mut sorted_chars: Vec<&CharacterStatus> = map.values().collect();

    sorted_chars.sort_by_key(|c| c.rank);

    for CharacterStatus { name, level, class, gold, mushrooms, rank, status } in sorted_chars {
        let (n, l, c, g, m, r, s) = (name, level, class, gold, mushrooms, rank, status);

        let _ = writeln!(
            buffer,
            "| {n:<w0$} | {l:>w1$} | {c:<w2$} | {g:>w3$.2} | {m:>w4$} | {r:>w5$} | {s:<w6$} |",
            w0 = WIDTHS.0,
            w1 = WIDTHS.1,
            w2 = WIDTHS.2,
            w3 = WIDTHS.3,
            w4 = WIDTHS.4,
            w5 = WIDTHS.5,
            w6 = WIDTHS.6
        );
    }

    let _ = writeln!(buffer, "{}", border);

    buffer
}

async fn print_character_table(shared_map: &SharedStatusMap) {
    let map = shared_map.lock().await;

    if map.is_empty() {
        return;
    }

    print!("{}", format_character_table(&map));
}

async fn process_session(mut session: SimpleSession, shared_map: SharedStatusMap) {
    if let Err(err) = session.send_command(Command::Update).await {
        return eprintln!("ERROR UPDATING SESSION ({:?})", err);
    }

    update_character_status(&session, &shared_map).await;

    log::log(&session, "DOWNLOADED AND READY TO RUN");

    loop {
        let hour = chrono::Local::now().hour();

        if let Err(err) = session.send_command(Command::Update).await {
            log::log(&session, &format!("FAILED TO UPDATE SESSION ({:?})", err));

            wait_between_actions().await;

            continue;
        }

        update_character_status(&session, &shared_map).await;

        guard(&mut session).await;

        if hour < constant::EXPEDITION_START_HOUR {
            wait_between_actions().await;
        }

        inventory(&mut session).await;

        let Some(gs) = session.game_state() else {
            continue;
        };

        if gs.character.inventory.count_free_slots() == 0 {
            log::log(&session, "FULL INVENTORY, SKIPPING EXPEDITIONS, DUNGEONS AND DAILY REWARDS");

            wait_between_actions().await;

            continue;
        }

        daily(&mut session).await;
        guild(&mut session).await;
        skill(&mut session).await;

        fortress(&mut session).await;

        dungeon(&mut session).await;

        arena(&mut session).await;

        let Some(gs) = session.game_state() else {
            continue;
        };

        let thirst = gs.tavern.thirst_for_adventure_sec;

        if gs.tavern.current_action == CurrentAction::Expedition || thirst > 0 {
            expedition(&mut session).await;
        }

        update_character_status(&session, &shared_map).await;

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (10000.0, 1200.0, 8000.0, 15000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
