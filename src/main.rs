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
mod mount;
mod pets;
mod shop;
mod skill;
mod witch;

use arena::arena;
use daily::daily;
use dungeon::dungeon;
use expedition::expedition;
use fortress::{fortress, underworld};
use guard::guard;
use guild::guild;
use inventory::inventory;
use mount::mount;
use pets::pets;
use shop::shop;
use skill::skill;
use witch::witch;

#[derive(Debug, Clone)]
struct CharacterStatus {
    name: String,
    guild: String,
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
    let args: Vec<String> = env::args().collect();

    let hidden = args.iter().any(|arg| arg == "--hidden");

    let positional: Vec<&String> = args.iter().skip(1).filter(|arg| *arg != "--hidden").collect();

    if positional.len() < 2 {
        let prog = args.first().map(|s| s.as_str()).unwrap_or("rsfb");

        eprintln!("USAGE: {} <USERNAME> <PASSWORD> [--hidden]", prog);

        process::exit(1);
    }

    log::set_hidden(hidden);

    let user = positional[0].clone();
    let pass = positional[1].clone();

    let sessions = match SimpleSession::login_sf_account(&user, &pass).await {
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

    tokio::time::sleep(Duration::from_millis(500)).await;

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
        tokio::time::sleep(Duration::from_millis(100)).await;

        let (u, p, sm) = (user.clone(), pass.clone(), shared_map.clone());

        handles.push(tokio::spawn(process_session(session, u, p, sm)));
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

        let guild = gs.guild.as_ref().map(|g| g.name.clone()).unwrap_or_else(|| "-".to_string());

        let mut map = shared_map.lock().await;

        let status = CharacterStatus {
            name: gs.character.name.clone(),
            guild: guild,
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

    const WIDTHS: (usize, usize, usize, usize, usize, usize, usize, usize) =
        (14, 16, 5, 14, 11, 7, 7, 13);

    let border = format!(
        "+{}+{}+{}+{}+{}+{}+{}+{}+",
        "-".repeat(WIDTHS.0 + 2),
        "-".repeat(WIDTHS.1 + 2),
        "-".repeat(WIDTHS.2 + 2),
        "-".repeat(WIDTHS.3 + 2),
        "-".repeat(WIDTHS.4 + 2),
        "-".repeat(WIDTHS.5 + 2),
        "-".repeat(WIDTHS.6 + 2),
        "-".repeat(WIDTHS.7 + 2)
    );

    let header = ("CHARACTER NAME", "GUILD", "LEVEL", "CLASS", "GOLD", "SHROOMS", "RANK", "STATUS");

    let _ = writeln!(buffer, "{}", border);

    let _ = writeln!(
        buffer,
        "| {:<w0$} | {:<w1$} | {:<w2$} | {:<w3$} | {:<w4$} | {:<w5$} | {:<w6$} | {:<w7$} |",
        header.0,
        header.1,
        header.2,
        header.3,
        header.4,
        header.5,
        header.6,
        header.7,
        w0 = WIDTHS.0,
        w1 = WIDTHS.1,
        w2 = WIDTHS.2,
        w3 = WIDTHS.3,
        w4 = WIDTHS.4,
        w5 = WIDTHS.5,
        w6 = WIDTHS.6,
        w7 = WIDTHS.7
    );

    let _ = writeln!(buffer, "{}", border);

    let mut s_chars: Vec<&CharacterStatus> = map.values().collect();

    s_chars.sort_by_key(|c| c.rank);

    for CharacterStatus { name, guild, level, class, gold, mushrooms, rank, status } in s_chars {
        let dn = if log::is_hidden() { "****" } else { name.as_str() };

        let dg = if log::is_hidden() { "****" } else { guild.as_str() };

        let (n, g, l, c, gd, m, r, s) = (dn, dg, level, class, gold, mushrooms, rank, status);

        let _ = writeln!(
            buffer,
            "| {n:<w0$} | {g:<w1$} | {l:>w2$} | {c:<w3$} | {gd:>w4$.2} | {m:>w5$} | {r:>w6$} | {s:<w7$} |",
            w0 = WIDTHS.0,
            w1 = WIDTHS.1,
            w2 = WIDTHS.2,
            w3 = WIDTHS.3,
            w4 = WIDTHS.4,
            w5 = WIDTHS.5,
            w6 = WIDTHS.6,
            w7 = WIDTHS.7
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

async fn relogin(user: &str, pass: &str, name: &str) -> Option<SimpleSession> {
    let (u, p, n) = (user, pass, name);

    SimpleSession::login_sf_account(u, p).await.ok()?.into_iter().find(|s| s.username() == n)
}

async fn process_session(mut sess: SimpleSession, user: String, pass: String, sm: SharedStatusMap) {
    let name = sess.username().to_string();

    if let Err(err) = sess.send_command(Command::Update).await {
        log::log(&sess, &format!("FAILED TO INITIALIZE SESSION ({:?})", err));

        if let Some(s) = relogin(&user, &pass, &name).await {
            sess = s;
        }
    }

    update_character_status(&sess, &sm).await;

    loop {
        let hour = chrono::Local::now().hour();

        if let Err(err) = sess.send_command(Command::Update).await {
            log::log(&sess, &format!("FAILED TO UPDATE SESSION ({:?})", err));

            tokio::time::sleep(Duration::from_secs(300)).await;

            if let Some(s) = relogin(&user, &pass, &name).await {
                log::log(&sess, "SSO CREDENTIALS RENEWED");

                sess = s;

                continue;
            }

            continue;
        }

        update_character_status(&sess, &sm).await;

        let Some(gs) = sess.game_state() else {
            continue;
        };

        if let Some(unlockable) = gs.pending_unlocks.first().copied() {
            log::log(&sess, &format!("UNLOCKING '{:?}' FEATURE", unlockable));

            if let Err(err) = sess.send_command(Command::UnlockFeature { unlockable }).await {
                log::log(&sess, &format!("FAILED TO UNLOCK FEATURE ({:?})", err));
            }

            wait_between_actions().await;

            continue;
        }

        guard(&mut sess).await;

        if hour < constant::EXPEDITION_START_HOUR {
            wait_between_actions().await;

            continue;
        }

        shop(&mut sess).await;

        inventory(&mut sess).await;

        fortress(&mut sess).await;

        underworld(&mut sess).await;

        let Some(gs) = sess.game_state() else {
            continue;
        };

        if gs.character.inventory.count_free_slots() == 0 {
            log::log(&sess, "FULL INVENTORY, SKIPPING EXPEDITIONS, DUNGEONS AND DAILY REWARDS");

            wait_between_actions().await;

            continue;
        }

        daily(&mut sess).await;
        guild(&mut sess).await;
        skill(&mut sess).await;
        mount(&mut sess).await;
        witch(&mut sess).await;
        arena(&mut sess).await;

        dungeon(&mut sess).await;

        pets(&mut sess).await;

        let Some(gs) = sess.game_state() else {
            continue;
        };

        let thirst = gs.tavern.thirst_for_adventure_sec;

        let can_drink_beer = expedition::can_drink_beer(&sess);

        if gs.tavern.current_action == CurrentAction::Expedition || thirst > 0 || can_drink_beer {
            expedition(&mut sess).await;
        }

        update_character_status(&sess, &sm).await;

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (10000.0, 1200.0, 8000.0, 15000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
