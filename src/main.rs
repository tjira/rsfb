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

use sf_api::{
    command::Command,
    gamestate::{character::Mount, tavern::CurrentAction},
    session::SimpleSession,
};

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
    mount: u16,
    treasure: f64,
    instructor: f64,
    thirst: u32,
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

        let guild_name =
            gs.guild.as_ref().map(|g| g.name.clone()).unwrap_or_else(|| "-".to_string());

        let now = chrono::Local::now();

        let active_mount = gs.character.mount.and_then(|m| match gs.character.mount_end {
            Some(end) if end <= now => None,

            _ => Some(m),
        });

        let mount = match active_mount {
            Some(Mount::Dragon) => 50,
            Some(Mount::Tiger) => 30,
            Some(Mount::Horse) => 20,
            Some(Mount::Cow) => 10,
            None => 0,
        };

        let (treasure, instructor) = match &gs.guild {
            Some(guild) => {
                let raid_bonus = (guild.finished_raids.min(50) * 2) as f64;

                let tre = ((guild.total_treasure_skill as f64 / 5.0) + raid_bonus).min(200.0);
                let i = ((guild.total_instructor_skill as f64 / 5.0) + raid_bonus).min(200.0);

                (tre, i)
            }

            None => (0.0, 0.0),
        };

        let thirst = gs.tavern.thirst_for_adventure_sec / 60;

        let mut map = shared_map.lock().await;

        let status = CharacterStatus {
            name: gs.character.name.clone(),
            guild: guild_name,
            level: gs.character.level,
            class: log::get_class_name(gs.character.class).to_string(),
            gold: (gs.character.silver as f64) / 100.0,
            mushrooms: gs.character.mushrooms,
            mount,
            treasure,
            instructor,
            thirst,
            rank: gs.character.rank,
            status,
        };

        map.insert(gs.character.name.clone(), status);
    }
}

fn format_character_table(map: &HashMap<String, CharacterStatus>) -> String {
    let mut buffer = String::new();

    const WIDTHS: (
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
    ) = (2, 14, 16, 5, 14, 11, 7, 5, 8, 10, 6, 7, 13);

    let border = format!(
        "+{}+{}+{}+{}+{}+{}+{}+{}+{}+{}+{}+{}+{}+",
        "-".repeat(WIDTHS.0 + 2),
        "-".repeat(WIDTHS.1 + 2),
        "-".repeat(WIDTHS.2 + 2),
        "-".repeat(WIDTHS.3 + 2),
        "-".repeat(WIDTHS.4 + 2),
        "-".repeat(WIDTHS.5 + 2),
        "-".repeat(WIDTHS.6 + 2),
        "-".repeat(WIDTHS.7 + 2),
        "-".repeat(WIDTHS.8 + 2),
        "-".repeat(WIDTHS.9 + 2),
        "-".repeat(WIDTHS.10 + 2),
        "-".repeat(WIDTHS.11 + 2),
        "-".repeat(WIDTHS.12 + 2)
    );

    let header = (
        "#",
        "CHARACTER NAME",
        "GUILD",
        "LEVEL",
        "CLASS",
        "GOLD",
        "SHROOMS",
        "MOUNT",
        "TREASURE",
        "INSTRUCTOR",
        "THIRST",
        "RANK",
        "STATUS",
    );

    let _ = writeln!(buffer, "{}", border);

    let _ = writeln!(
        buffer,
        "| {:<w0$} | {:<w1$} | {:<w2$} | {:<w3$} | {:<w4$} | {:<w5$} | {:<w6$} | {:<w7$} | {:<w8$} | {:<w9$} | {:<w10$} | {:<w11$} | {:<w12$} |",
        header.0,
        header.1,
        header.2,
        header.3,
        header.4,
        header.5,
        header.6,
        header.7,
        header.8,
        header.9,
        header.10,
        header.11,
        header.12,
        w0 = WIDTHS.0,
        w1 = WIDTHS.1,
        w2 = WIDTHS.2,
        w3 = WIDTHS.3,
        w4 = WIDTHS.4,
        w5 = WIDTHS.5,
        w6 = WIDTHS.6,
        w7 = WIDTHS.7,
        w8 = WIDTHS.8,
        w9 = WIDTHS.9,
        w10 = WIDTHS.10,
        w11 = WIDTHS.11,
        w12 = WIDTHS.12
    );

    let _ = writeln!(buffer, "{}", border);

    let mut s_chars: Vec<&CharacterStatus> = map.values().collect();

    s_chars.sort_by_key(|c| c.rank);

    for (
        i,
        CharacterStatus {
            name,
            guild,
            level,
            class,
            gold,
            mushrooms,
            mount,
            treasure,
            instructor,
            thirst,
            rank,
            status,
        },
    ) in s_chars.into_iter().enumerate()
    {
        let dn = if log::is_hidden() { "****" } else { name.as_str() };

        let dg = if log::is_hidden() { "****" } else { guild.as_str() };

        let (idx, n, g, l, c, gd, m, mt, tr, ins, th, r, s) = (
            i + 1,
            dn,
            dg,
            level,
            class,
            gold,
            mushrooms,
            format!("{mount}%"),
            format!("{treasure:.1}%"),
            format!("{instructor:.1}%"),
            thirst,
            rank,
            status,
        );

        let _ = writeln!(
            buffer,
            "| {idx:>w0$} | {n:<w1$} | {g:<w2$} | {l:>w3$} | {c:<w4$} | {gd:>w5$.2} | {m:>w6$} | {mt:>w7$} | {tr:>w8$} | {ins:>w9$} | {th:>w10$} | {r:>w11$} | {s:<w12$} |",
            w0 = WIDTHS.0,
            w1 = WIDTHS.1,
            w2 = WIDTHS.2,
            w3 = WIDTHS.3,
            w4 = WIDTHS.4,
            w5 = WIDTHS.5,
            w6 = WIDTHS.6,
            w7 = WIDTHS.7,
            w8 = WIDTHS.8,
            w9 = WIDTHS.9,
            w10 = WIDTHS.10,
            w11 = WIDTHS.11,
            w12 = WIDTHS.12
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

    if let Err(_) = sess.send_command(Command::Update).await {
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
