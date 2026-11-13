//! Officer rank: a character given command of the fleet or system it is at,
//! through the object popup's Command submenu (orders `0x260..0x263`;
//! `ghidra/notes/character-roster.md`, "Officer rank").

use serde::{Deserialize, Serialize};

use crate::ids::{CharacterKey, FleetKey, SystemKey};
use crate::world::{Character, GameWorld};

/// The character short `+0x96` (CommandKind; setter `FUN_004ee350`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CommandKind {
    #[default]
    None,
    Commander,
    Admiral,
    General,
}

impl CommandKind {
    /// The kind a Command submenu order sets: `0x260` None, `0x261`
    /// Commander, `0x262` Admiral, `0x263` General (`FUN_00561bb0`).
    #[must_use]
    pub const fn from_order(command: u16) -> Option<Self> {
        match command {
            0x260 => Some(Self::None),
            0x261 => Some(Self::Commander),
            0x262 => Some(Self::Admiral),
            0x263 => Some(Self::General),
            _ => None,
        }
    }

    /// The submenu order that sets this kind (`FUN_0053ebb0` maps
    /// `0x260..0x263` to 0..3).
    #[must_use]
    pub const fn order(self) -> u16 {
        match self {
            Self::None => 0x260,
            Self::Commander => 0x261,
            Self::Admiral => 0x262,
            Self::General => 0x263,
        }
    }

    /// The TEXTSTRA block a ranked name adds to the character's name id
    /// (`FUN_004f17f0`: "Ackbar" 10816, "Admiral Ackbar" 38464).
    #[must_use]
    pub const fn name_block(self) -> Option<u16> {
        match self {
            Self::None => None,
            Self::Commander => Some(0x6800),
            Self::Admiral => Some(0x6c00),
            Self::General => Some(0x7000),
        }
    }

    /// Index of this kind's name in [`Character::ranked_names`]. port: the
    /// port keeps the three ranked TEXTSTRA names on the character.
    #[must_use]
    pub const fn name_index(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Commander => Some(0),
            Self::Admiral => Some(1),
            Self::General => Some(2),
        }
    }
}

/// What a rank commands (`+0xa8`, `FUN_004ee740`): the character's fleet,
/// or the system it stands at (`FUN_0055ed30`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandPost {
    Fleet(FleetKey),
    System(SystemKey),
}

/// The post a character would command where it is now (`FUN_0055ed30`).
/// port: a character rides a fleet, not one capital ship in it, so the
/// fleet is the post whichever ship the original would name.
#[must_use]
pub const fn command_post(character: &Character) -> Option<CommandPost> {
    match (character.current_fleet, character.current_system) {
        (Some(fleet), _) => Some(CommandPost::Fleet(fleet)),
        (None, Some(system)) => Some(CommandPost::System(system)),
        (None, None) => None,
    }
}

/// Why a Command submenu order is refused for a character: the object
/// check `FUN_004f9860`, then `FUN_004ed750`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRefusal {
    /// Not the ordering side's (`+0x24` bits 6..7).
    OtherSide,
    /// Not recruited (`+0x50` bit 2).
    Unrecruited,
    /// Travelling (`+0x50` bit 4): standing at its destination until it
    /// arrives.
    EnRoute,
    /// On a mission (`+0x78` bit 7).
    OnMission,
    /// Held prisoner (`+0xac` bit 0).
    Prisoner,
    /// Admiral for a character that is not aboard a fleet: `FUN_0055f160`
    /// refuses kind 2 with status `(0x30, 5)` when `FUN_0055ed30` finds no
    /// fleet above the character.
    NotAboard,
    /// None with no rank held, or a rank its `can_be_*` flag denies.
    NotEligible,
}

/// Whether `side_is_alliance` may give `character` the rank `kind`.
/// port: the port keeps no injury (`+0x94`), so that refusal never fires.
#[must_use]
pub fn command_refusal(
    world: &GameWorld,
    character: CharacterKey,
    side_is_alliance: bool,
    kind: CommandKind,
) -> Option<CommandRefusal> {
    let Some(c) = world.characters.get(character) else {
        return Some(CommandRefusal::OtherSide);
    };
    let own_side = if side_is_alliance {
        c.is_alliance
    } else {
        c.is_empire
    };
    if !own_side || c.is_killed {
        return Some(CommandRefusal::OtherSide);
    }
    if !c.recruited {
        return Some(CommandRefusal::Unrecruited);
    }
    if c.on_mission || c.on_hidden_mission {
        return Some(CommandRefusal::OnMission);
    }
    if c.is_captive {
        return Some(CommandRefusal::Prisoner);
    }
    if c.en_route || command_post(c).is_none() {
        return Some(CommandRefusal::EnRoute);
    }
    // FUN_0055f160 (0x55f2a0): an Admiral needs a fleet, which
    // FUN_0055ed30 finds only through a ship the character rides.
    if kind == CommandKind::Admiral && c.current_fleet.is_none() {
        return Some(CommandRefusal::NotAboard);
    }
    let eligible = match kind {
        CommandKind::None => c.command_kind != CommandKind::None,
        CommandKind::Commander => c.can_be_commander,
        CommandKind::Admiral => c.can_be_admiral,
        CommandKind::General => c.can_be_general,
    };
    (!eligible).then_some(CommandRefusal::NotEligible)
}

/// The side's character holding `kind` at `post`: in the fleet
/// (`FUN_004fd790` → `FUN_005006f0`) or standing at the system
/// (`FUN_00509330`). hyp: the system walk's side (`+0x24` bits 6..7 of the
/// system) is the ordering character's side.
#[must_use]
pub fn rank_holder(
    world: &GameWorld,
    post: CommandPost,
    kind: CommandKind,
    is_alliance: bool,
) -> Option<CharacterKey> {
    world.characters.iter().find_map(|(key, c)| {
        (c.command_kind == kind
            && kind != CommandKind::None
            && c.is_alliance == is_alliance
            && !c.is_killed
            && command_post(c) == Some(post))
        .then_some(key)
    })
}

/// Carry out a Command submenu order the caller has checked with
/// [`command_refusal`] (`FUN_00561bb0` → `FUN_004ef340` → `FUN_004ef400`):
/// the rank the character holds toggles off; any other rank is given, and
/// whoever of the side held it at the same post loses it.
pub fn give_command(world: &mut GameWorld, character: CharacterKey, kind: CommandKind) {
    let Some(c) = world.characters.get(character) else {
        return;
    };
    let post = command_post(c);
    let is_alliance = c.is_alliance;
    if kind == CommandKind::None || c.command_kind == kind {
        set_command(world, character, CommandKind::None, None);
        return;
    }
    let holder = post.and_then(|post| rank_holder(world, post, kind, is_alliance));
    if let Some(holder) = holder.filter(|&holder| holder != character) {
        set_command(world, holder, CommandKind::None, None);
    }
    set_command(world, character, kind, post);
}

/// The setter `FUN_004ee350` and the commanded key `FUN_004ee740`.
fn set_command(
    world: &mut GameWorld,
    character: CharacterKey,
    kind: CommandKind,
    post: Option<CommandPost>,
) {
    if let Some(c) = world.characters.get_mut(character) {
        c.command_kind = kind;
        c.command_post = post;
    }
}

/// Take the rank from every holder who may no longer hold it: a prisoner
/// (`+0xac` bit 0) or on a mission (its `+0x68` key in families
/// `0x50..0x7f`) (`FUN_004eeed0`), or moved off the post it was given at
/// (`FUN_00560d50`). A fleet keeps its key while it travels, so its
/// officers keep their ranks, as the original's do. hyp: `FUN_004eeed0`'s
/// `+0x50` bit 6 test is "recruited"; a killed character's object is
/// destroyed with its rank.
/// port: the original clears on each change notification; the game sweeps
/// once a frame, after the tick and before the player's orders are drawn,
/// and the headless runner after each tick.
pub fn clear_lapsed_ranks(world: &mut GameWorld) {
    for (_, c) in &mut world.characters {
        if c.command_kind == CommandKind::None {
            continue;
        }
        let lapsed = c.is_killed
            || c.is_captive
            || c.on_mission
            || c.on_hidden_mission
            || !c.recruited
            || command_post(c) != c.command_post;
        if lapsed {
            c.command_kind = CommandKind::None;
            c.command_post = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Fleet, System};

    /// A recruited character of `is_alliance`'s side able to hold every rank,
    /// standing at a system or riding a fleet there.
    fn officer(aboard: bool, is_alliance: bool) -> (GameWorld, CharacterKey) {
        let mut world = GameWorld::default();
        let system = world.systems.insert(System::default());
        let fleet = world.fleets.insert(Fleet {
            location: system,
            is_alliance,
            ..Fleet::default()
        });
        let character = world.characters.insert(Character {
            is_alliance,
            is_empire: !is_alliance,
            recruited: true,
            can_be_admiral: true,
            can_be_commander: true,
            can_be_general: true,
            current_system: Some(system),
            current_fleet: aboard.then_some(fleet),
            ..Character::default()
        });
        (world, character)
    }

    /// Every gate of `FUN_004f9860` -> `FUN_004ed750` refuses on its own
    /// condition, for either side: a rank is reached only once all of them
    /// pass. A character standing at a system may be Commander.
    #[test]
    fn each_command_gate_refuses_on_its_own_condition() {
        for is_alliance in [true, false] {
            let refused = |edit: fn(&mut Character)| {
                let (mut world, character) = officer(false, is_alliance);
                edit(&mut world.characters[character]);
                command_refusal(&world, character, is_alliance, CommandKind::Commander)
            };
            assert_eq!(refused(|_| {}), None, "{is_alliance}");
            // +0x24 bits 6..7: the other side's, and the dead.
            assert_eq!(
                refused(|c| {
                    c.is_alliance = !c.is_alliance;
                    c.is_empire = !c.is_empire;
                }),
                Some(CommandRefusal::OtherSide)
            );
            assert_eq!(
                refused(|c| c.is_killed = true),
                Some(CommandRefusal::OtherSide)
            );
            assert_eq!(
                refused(|c| c.recruited = false),
                Some(CommandRefusal::Unrecruited)
            );
            // Either mission flag is enough (+0x78 bit 8 hides the mission).
            assert_eq!(
                refused(|c| c.on_mission = true),
                Some(CommandRefusal::OnMission)
            );
            assert_eq!(
                refused(|c| c.on_hidden_mission = true),
                Some(CommandRefusal::OnMission)
            );
            assert_eq!(
                refused(|c| c.is_captive = true),
                Some(CommandRefusal::Prisoner)
            );
            // En route, or at no post at all: either refuses.
            assert_eq!(
                refused(|c| c.en_route = true),
                Some(CommandRefusal::EnRoute)
            );
            assert_eq!(
                refused(|c| c.current_system = None),
                Some(CommandRefusal::EnRoute)
            );
            assert_eq!(
                refused(|c| c.can_be_commander = false),
                Some(CommandRefusal::NotEligible)
            );
        }
    }

    /// Taking a rank away (kind 0) needs one held: the order is refused for
    /// a character who holds none.
    #[test]
    fn the_none_rank_is_offered_only_to_a_character_who_holds_one() {
        for is_alliance in [true, false] {
            let (mut world, character) = officer(false, is_alliance);
            assert_eq!(
                command_refusal(&world, character, is_alliance, CommandKind::None),
                Some(CommandRefusal::NotEligible),
                "{is_alliance}: no rank to take away"
            );
            world.characters[character].command_kind = CommandKind::Commander;
            assert_eq!(
                command_refusal(&world, character, is_alliance, CommandKind::None),
                None
            );
        }
    }

    /// `FUN_0055f160` (0x55f2a0) refuses kind 2 with `(0x30, 5)` when
    /// `FUN_0055ed30` finds no fleet above the character; Commander and
    /// General stay open at a system.
    #[test]
    fn an_admiral_must_ride_a_fleet_on_either_side() {
        for is_alliance in [true, false] {
            let (world, character) = officer(false, is_alliance);
            let refusal = |kind| command_refusal(&world, character, is_alliance, kind);
            assert_eq!(refusal(CommandKind::Admiral), Some(CommandRefusal::NotAboard));
            assert_eq!(refusal(CommandKind::General), None);
            assert_eq!(refusal(CommandKind::Commander), None);

            let (world, character) = officer(true, is_alliance);
            assert_eq!(
                command_refusal(&world, character, is_alliance, CommandKind::Admiral),
                None
            );
        }
    }
}
