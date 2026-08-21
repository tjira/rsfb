use std::sync::atomic::{AtomicBool, Ordering};

use sf_api::{gamestate::character::Class, session::SimpleSession};

static HIDDEN: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_hidden(hidden: bool) {
    HIDDEN.store(hidden, Ordering::Relaxed);
}

pub(crate) fn is_hidden() -> bool {
    HIDDEN.load(Ordering::Relaxed)
}

pub(crate) fn get_class_name(class: Class) -> &'static str {
    match class {
        Class::Assassin => "Assassin",
        Class::Bard => "Bard",
        Class::BattleMage => "Battle Mage",
        Class::Berserker => "Berserker",
        Class::BloodWeaver => "Blood Weaver",
        Class::DemonHunter => "Demon Hunter",
        Class::Druid => "Druid",
        Class::Mage => "Mage",
        Class::Necromancer => "Necromancer",
        Class::Paladin => "Paladin",
        Class::PlagueDoctor => "Plague Doctor",
        Class::Scout => "Scout",
        Class::Warrior => "Warrior",
    }
}

pub(crate) fn log(session: &SimpleSession, message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    if let Some(gs) = session.game_state() {
        let (name, level, class) = {
            let name = if is_hidden() { "****" } else { &gs.character.name };

            let (name, level) = (name.to_string(), gs.character.level);

            (name, level, get_class_name(gs.character.class))
        };

        return println!("[{timestamp}] Level {level} {class} ({name}): {message}");
    }

    let username = if is_hidden() { "****" } else { session.username() };

    println!("[{timestamp}] USER '{username}' (STATE UNPOPULATED): {message}");
}
