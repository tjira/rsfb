use sf_api::error::SFError;
use sf_api::gamestate::character::Class;
use sf_api::session::SimpleSession;

fn get_class_name(class: Class) -> &'static str {
    match class {
        Class::Assassin => "Assassin",
        Class::Bard => "Bard",
        Class::BattleMage => "Battle Mage",
        Class::Berserker => "Berserker",
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

pub(crate) fn log(session: &SimpleSession, message: &str) -> Result<(), sf_api::error::SFError> {
    let gs = session.game_state().ok_or_else(|| SFError::InvalidRequest("GAME STATE IS NOT POPULATED"))?;

    let (name, level, class) = {
        (gs.character.name.clone(), gs.character.level, get_class_name(gs.character.class))
    };

    println!("Level {level} {class} ({name}): {message}");

    Ok(())
}
