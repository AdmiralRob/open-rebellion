//! The Agent menu (`ghidra/notes/agent-menu.md`): STRATEGY `RT_RCDATA` list
//! `0xdead`, items `0x110..0x11e`. A right click on the advisor panel opens
//! it (`FUN_00422ce0` → `FUN_004420b0` → `FUN_0042d050`), and
//! `FUN_00487900` enables and checks its items before it shows.

use egui_macroquad::egui;

use crate::bmp_cache::BmpCache;
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::game_menu::{
    draw_game_menu, game_menu_rects, GameMenuEntry, GameMenuPlacement, GameMenuResponse,
    OwnedMenuOutcome,
};

const AGENT_MENU_ID: &str = "original-agent-menu";

/// The bitmap each record names in word 10, drawn only for a checked item
/// (`FUN_004abe30`, `FUN_004aba60`: flags bit 1).
const CHECK_BITMAP: u32 = 11902;

/// An Agent menu command the port carries out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCommand {
    /// 0x115, Alt+G: toggle module 0x15 (`FUN_00439d60`).
    ManageGarrisons,
    /// 0x116, Alt+U: toggle module 0x14 (`FUN_00439d60`).
    ManageProduction,
    /// 0x11e, Alt+A: flip bit `0x8000` (`FUN_00439e80`).
    AgentAdvice,
}

/// One record of list `0xdead`: its command, sort key (word 2) and TEXTSTRA
/// label (word 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentMenuRecord {
    pub command: u16,
    pub sort_key: u16,
    pub label: &'static str,
}

/// STRATEGY `RT_RCDATA` `0x110..0x11e` (no `0x11d` in the list), in sort
/// order; labels are TEXTSTRA `0x3000 + command`.
pub const AGENT_MENU_RECORDS: [AgentMenuRecord; 14] = [
    AgentMenuRecord {
        command: 0x110,
        sort_key: 100,
        label: "Build Ships",
    },
    AgentMenuRecord {
        command: 0x111,
        sort_key: 110,
        label: "Build Troops",
    },
    AgentMenuRecord {
        command: 0x112,
        sort_key: 120,
        label: "Build Facilities",
    },
    AgentMenuRecord {
        command: 0x113,
        sort_key: 200,
        label: "Galaxy Overview",
    },
    AgentMenuRecord {
        command: 0x114,
        sort_key: 500,
        label: "Objectives",
    },
    AgentMenuRecord {
        command: 0x115,
        sort_key: 510,
        label: "Manage Garrisons",
    },
    AgentMenuRecord {
        command: 0x116,
        sort_key: 520,
        label: "Manage Production",
    },
    AgentMenuRecord {
        command: 0x117,
        sort_key: 530,
        label: "Manage Maintenance",
    },
    AgentMenuRecord {
        command: 0x118,
        sort_key: 540,
        label: "Not An Operation",
    },
    AgentMenuRecord {
        command: 0x119,
        sort_key: 600,
        label: "Deactivate",
    },
    AgentMenuRecord {
        command: 0x11a,
        sort_key: 601,
        label: "Reactivate",
    },
    AgentMenuRecord {
        command: 0x11b,
        sort_key: 610,
        label: "Translate Counterpart",
    },
    AgentMenuRecord {
        command: 0x11e,
        sort_key: 615,
        label: "Agent Advice",
    },
    AgentMenuRecord {
        command: 0x11c,
        sort_key: 620,
        label: "Messages",
    },
];

/// The agent state the menu shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgentMenuView {
    pub garrisons: bool,
    pub production: bool,
    pub advice: bool,
}

/// One item as `FUN_00487900` leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentMenuItem {
    pub record: AgentMenuRecord,
    pub enabled: bool,
    pub checked: bool,
    pub command: Option<AgentCommand>,
}

/// The items `FUN_00487900` enables and checks. Manage Garrisons and Manage
/// Production are checked while they run (`FUN_00439e30`), Agent Advice
/// while bit `0x8000` is clear. port: the Build orders (module 0x17), the
/// encyclopedia views, Translate Counterpart and Messages are not ported, so
/// they stay disabled, as are the items `FUN_00487900` leaves alone.
#[must_use]
pub fn agent_menu_items(view: AgentMenuView) -> Vec<AgentMenuItem> {
    AGENT_MENU_RECORDS
        .iter()
        .map(|&record| {
            let (command, checked) = match record.command {
                0x115 => (Some(AgentCommand::ManageGarrisons), view.garrisons),
                0x116 => (Some(AgentCommand::ManageProduction), view.production),
                0x11e => (Some(AgentCommand::AgentAdvice), view.advice),
                _ => (None, false),
            };
            AgentMenuItem {
                record,
                enabled: command.is_some(),
                checked,
                command,
            }
        })
        .collect()
}

/// Whether a right click at canvas `point` lands on the agent's panel.
/// hyp: the agent is the larger droid aperture (`FUN_0042adb0`, panel
/// `param_1[0x4a]`); the other panel's list is `0xbeef`, the Message Alerts.
#[must_use]
pub fn agent_panel_contains(faction: CockpitFaction, point: (f32, f32)) -> bool {
    let (x, y, width, height) = crate::advisor::agent_aperture(faction);
    point.0 >= x && point.0 < x + width && point.1 >= y && point.1 < y + height
}

/// The open menu's anchor, a 640 by 480 canvas point.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AgentMenuState {
    pub anchor: Option<(f32, f32)>,
}

/// Canvas anchor for a routed secondary release over the agent panel.
#[must_use]
pub fn agent_menu_anchor(
    layout: CockpitLayout,
    faction: CockpitFaction,
    point: (f32, f32),
) -> Option<(f32, f32)> {
    (layout.scale > 0.0).then_some(())?;
    let logical = (
        (point.0 - layout.canvas.x) / layout.scale,
        (point.1 - layout.canvas.y) / layout.scale,
    );
    agent_panel_contains(faction, logical).then_some(logical)
}

/// The list a droid panel's right click opens (`FUN_004420b0` returns the
/// hit panel's own id, which `FUN_0042d050` fills).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroidMenuList {
    /// `0xdead`, the agent droid's: `FUN_0041d160` → `FUN_00487900`.
    Agent,
    /// `0xbeef`, the partner droid's: `FUN_0041d190` → `FUN_00487a60`.
    MessageAlerts,
}

/// Whether a right click at canvas `point` lands on the partner's panel.
/// hyp: the partner is the smaller droid aperture (`FUN_0042adb0`, panel
/// `param_1[0x4b]`).
#[must_use]
pub fn partner_panel_contains(faction: CockpitFaction, point: (f32, f32)) -> bool {
    crate::cockpit::partner_droid_command_at(faction, point).is_some()
}

/// The list a right click at canvas `point` opens: `FUN_00422ce0` on
/// `WM_RBUTTONUP` (`0x205`) tests the agent's panel, then the partner's.
#[must_use]
pub fn droid_menu_list_at(faction: CockpitFaction, point: (f32, f32)) -> Option<DroidMenuList> {
    if agent_panel_contains(faction, point) {
        Some(DroidMenuList::Agent)
    } else if partner_panel_contains(faction, point) {
        Some(DroidMenuList::MessageAlerts)
    } else {
        None
    }
}

/// Canvas anchor for a routed secondary release over either droid's panel;
/// [`droid_menu_list_at`] names the list the anchor opens.
#[must_use]
pub fn droid_menu_anchor(
    layout: CockpitLayout,
    faction: CockpitFaction,
    point: (f32, f32),
) -> Option<(f32, f32)> {
    (layout.scale > 0.0).then_some(())?;
    let logical = (
        (point.0 - layout.canvas.x) / layout.scale,
        (point.1 - layout.canvas.y) / layout.scale,
    );
    droid_menu_list_at(faction, logical).map(|_| logical)
}

/// One Message Alerts record: its command, the category mask its handler
/// passes to `FUN_0048a1c0` (`FUN_00486fb0`), and its TEXTSTRA label (the
/// record's word 5, `0x3120..0x3128`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageAlertRecord {
    pub command: u16,
    pub mask: u16,
    pub label: &'static str,
}

/// STRATEGY `RT_RCDATA` `0x171..0x179`, in the order `FUN_00487a60` adds
/// them, which is also their sort order (word 2: Chat `0x27e` before Advice
/// `0x27f`).
pub const MESSAGE_ALERT_RECORDS: [MessageAlertRecord; 9] = [
    MessageAlertRecord {
        command: 0x171,
        mask: 0x001,
        label: "Loyalty",
    },
    MessageAlertRecord {
        command: 0x172,
        mask: 0x080,
        label: "Fleets",
    },
    MessageAlertRecord {
        command: 0x173,
        mask: 0x010,
        label: "Mission",
    },
    MessageAlertRecord {
        command: 0x174,
        mask: 0x004,
        label: "Resources",
    },
    MessageAlertRecord {
        command: 0x175,
        mask: 0x008,
        label: "Manufacturing",
    },
    MessageAlertRecord {
        command: 0x176,
        mask: 0x040,
        label: "Defense",
    },
    MessageAlertRecord {
        command: 0x177,
        mask: 0x100,
        label: "Conflict",
    },
    MessageAlertRecord {
        command: 0x179,
        mask: 0x020,
        label: "Chat",
    },
    MessageAlertRecord {
        command: 0x178,
        mask: 0x200,
        label: "Advice",
    },
];

/// A Message Alerts menu command (`FUN_00486fb0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageAlertsCommand {
    /// `0x11c` Messages: `FUN_0041d770(1, 0x79)`, the Message Index on All.
    Messages,
    /// `0x171..0x179`: flip Post Messages Silently on the category `mask`
    /// (`FUN_0048a1c0`, then `FUN_0048a210` with the flags `^ 4`).
    ToggleAlert { mask: u16 },
}

/// One Message Alerts menu item as `FUN_00487a60` leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageAlertsItem {
    pub label: &'static str,
    pub checked: bool,
    pub command: MessageAlertsCommand,
}

/// The items `FUN_00487a60` adds, all enabled: Messages (`0x11c`), then
/// each category, checked while its posting flags lack Post Messages
/// Silently (`FUN_0048a1c0(mask) & 4 == 0`), that is while the droids
/// announce it. `post_flags` is `MessageLog::post_flags`.
#[must_use]
pub fn message_alerts_items(post_flags: impl Fn(u16) -> u32) -> Vec<MessageAlertsItem> {
    std::iter::once(MessageAlertsItem {
        label: "Messages",
        checked: false,
        command: MessageAlertsCommand::Messages,
    })
    .chain(
        MESSAGE_ALERT_RECORDS
            .iter()
            .map(|record| MessageAlertsItem {
                label: record.label,
                checked: post_flags(record.mask) & crate::message_log::POST_SILENTLY == 0,
                command: MessageAlertsCommand::ToggleAlert { mask: record.mask },
            }),
    )
    .collect()
}

/// A category's posting flags after its Message Alerts item is chosen
/// (`FUN_00486fb0`: `FUN_0048a210(mask, FUN_0048a1c0(mask) ^ 4)`).
#[must_use]
pub const fn toggled_alert_flags(flags: u32) -> u32 {
    flags ^ crate::message_log::POST_SILENTLY
}

/// Draw manager-owned Message Alerts content without deleting its payload.
/// It shares the Agent menu's identity: the two are one popup in the
/// original (`FUN_0042d050`) and never open together.
pub fn draw_owned_message_alerts_menu(
    ctx: &egui::Context,
    state: &AgentMenuState,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    items: &[MessageAlertsItem],
    input_enabled: bool,
) -> OwnedMenuOutcome<MessageAlertsCommand> {
    let Some(anchor) = state.anchor else {
        return OwnedMenuOutcome::Open;
    };
    let entries: Vec<GameMenuEntry<'_>> = items
        .iter()
        .map(|item| GameMenuEntry {
            label: item.label,
            icon: item.checked.then_some(CHECK_BITMAP),
            enabled: true,
            submenu: false,
            children: &[],
        })
        .collect();
    crate::control_registry::name_next_menu("MessageAlertsMenu");
    match draw_game_menu(
        ctx,
        egui::Id::new(AGENT_MENU_ID),
        cache,
        layout,
        faction,
        GameMenuPlacement::in_frame(anchor),
        &entries,
        input_enabled,
    ) {
        GameMenuResponse::Open => OwnedMenuOutcome::Open,
        GameMenuResponse::Dismissed | GameMenuResponse::ChosenChild { .. } => {
            OwnedMenuOutcome::Dismissed
        }
        GameMenuResponse::Chosen(index) => OwnedMenuOutcome::Chosen(items[index].command),
    }
}

/// Fixture-only direct-egui adapter. Production opens through the normalized
/// strategic pointer batch and the manager-owned Menu singleton.
#[cfg(test)]
pub fn open_agent_menu_on_right_click(
    ctx: &egui::Context,
    state: &mut AgentMenuState,
    layout: CockpitLayout,
    faction: CockpitFaction,
) -> bool {
    if layout.scale <= 0.0 || ctx.is_pointer_over_area() {
        return false;
    }
    let (released, pointer) = ctx.input(|input| {
        (
            input
                .pointer
                .button_released(egui::PointerButton::Secondary),
            input.pointer.interact_pos(),
        )
    });
    let Some(pointer) = pointer.filter(|_| released) else {
        return false;
    };
    let Some(anchor) = agent_menu_anchor(layout, faction, (pointer.x, pointer.y)) else {
        return false;
    };
    state.anchor = Some(anchor);
    true
}

/// Root and currently visible child rectangles of the open Agent menu.
#[must_use]
pub fn agent_menu_rects(ctx: &egui::Context) -> Vec<egui::Rect> {
    game_menu_rects(ctx, egui::Id::new(AGENT_MENU_ID))
}

/// Root and visible-submenu layers of the managed Agent menu.
#[must_use]
pub fn agent_menu_layer_ids(ctx: &egui::Context) -> Vec<egui::LayerId> {
    crate::game_menu::game_menu_layer_ids(ctx, egui::Id::new(AGENT_MENU_ID))
}

/// Draw manager-owned Agent-menu content without deleting its payload.
pub fn draw_owned_agent_menu(
    ctx: &egui::Context,
    state: &AgentMenuState,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    view: AgentMenuView,
    input_enabled: bool,
) -> OwnedMenuOutcome<AgentCommand> {
    let Some(anchor) = state.anchor else {
        return OwnedMenuOutcome::Open;
    };
    let items = agent_menu_items(view);
    let entries: Vec<GameMenuEntry<'_>> = items
        .iter()
        .map(|item| GameMenuEntry {
            label: item.record.label,
            icon: item.checked.then_some(CHECK_BITMAP),
            enabled: item.enabled,
            submenu: false,
            children: &[],
        })
        .collect();
    crate::control_registry::name_next_menu("AgentMenu");
    match draw_game_menu(
        ctx,
        egui::Id::new(AGENT_MENU_ID),
        cache,
        layout,
        faction,
        GameMenuPlacement::in_frame(anchor),
        &entries,
        input_enabled,
    ) {
        GameMenuResponse::Open => OwnedMenuOutcome::Open,
        GameMenuResponse::Dismissed | GameMenuResponse::ChosenChild { .. } => {
            OwnedMenuOutcome::Dismissed
        }
        GameMenuResponse::Chosen(index) => items[index]
            .command
            .map_or(OwnedMenuOutcome::Open, OwnedMenuOutcome::Chosen),
    }
}

/// Fixture-only wrapper retaining the original direct content lifecycle.
#[cfg(test)]
pub fn draw_agent_menu(
    ctx: &egui::Context,
    state: &mut AgentMenuState,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    view: AgentMenuView,
    input_enabled: bool,
) -> Option<AgentCommand> {
    match draw_owned_agent_menu(ctx, state, cache, layout, faction, view, input_enabled) {
        OwnedMenuOutcome::Open => None,
        OwnedMenuOutcome::Dismissed => {
            state.anchor = None;
            None
        }
        OwnedMenuOutcome::Chosen(command) => {
            state.anchor = None;
            Some(command)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_lists_list_0xdeads_records_in_sort_order() {
        // STRATEGY RT_RCDATA 0x110..0x11e, word 2 the sort key, word 5 the
        // TEXTSTRA label; 0x11d (Message Alerts) is not in the list.
        let keys: Vec<u16> = AGENT_MENU_RECORDS
            .iter()
            .map(|record| record.sort_key)
            .collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
        assert!(!AGENT_MENU_RECORDS
            .iter()
            .any(|record| record.command == 0x11d));
        assert_eq!(AGENT_MENU_RECORDS[12].label, "Agent Advice");
    }

    #[test]
    fn running_automations_and_advice_are_checked_and_enabled() {
        // FUN_00487900: 0x115/0x116 checked when FUN_00439e30 says the module
        // runs; 0x11e checked when bit 0x8000 is clear.
        let item = |items: &[AgentMenuItem], command| {
            *items
                .iter()
                .find(|item| item.record.command == command)
                .unwrap()
        };
        let off = agent_menu_items(AgentMenuView::default());
        for command in [0x115, 0x116, 0x11e] {
            assert!(item(&off, command).enabled && !item(&off, command).checked);
        }
        let on = agent_menu_items(AgentMenuView {
            garrisons: true,
            production: false,
            advice: true,
        });
        assert!(item(&on, 0x115).checked);
        assert!(!item(&on, 0x116).checked);
        assert!(item(&on, 0x11e).checked);
        assert_eq!(item(&on, 0x11e).command, Some(AgentCommand::AgentAdvice));
        assert!(!item(&on, 0x110).enabled, "port: Build Ships is not ported");
    }

    #[test]
    fn a_right_click_on_each_droid_opens_that_droids_own_list() {
        // FUN_00422ce0, WM_RBUTTONUP (0x205): FUN_004420b0 on param_1[0x4a],
        // then on param_1[0x4b]; FUN_0042d050 fills 0xdead through
        // FUN_0041d160 and 0xbeef through FUN_0041d190. Panels: FUN_0042adb0.
        for (faction, agent, partner) in [
            (CockpitFaction::Alliance, (560.0, 400.0), (340.0, 440.0)),
            (CockpitFaction::Empire, (50.0, 400.0), (350.0, 440.0)),
        ] {
            assert_eq!(
                droid_menu_list_at(faction, agent),
                Some(DroidMenuList::Agent)
            );
            assert_eq!(
                droid_menu_list_at(faction, partner),
                Some(DroidMenuList::MessageAlerts)
            );
            assert_eq!(droid_menu_list_at(faction, (200.0, 200.0)), None);
        }
    }

    #[test]
    fn a_droid_menu_anchor_is_the_click_in_canvas_points() {
        use crate::cockpit::CockpitViewport;
        let layout = |scale: f32| CockpitLayout {
            canvas: CockpitViewport {
                x: 10.0,
                y: 20.0,
                width: 640.0 * scale,
                height: 480.0 * scale,
            },
            galaxy: CockpitViewport {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            scale,
        };
        // SD-7 (302, 401, 101, 79) on a canvas at (10, 20), scale 2.
        assert_eq!(
            droid_menu_anchor(layout(2.0), CockpitFaction::Empire, (710.0, 900.0)),
            Some((350.0, 440.0))
        );
        assert_eq!(
            droid_menu_anchor(layout(2.0), CockpitFaction::Empire, (410.0, 420.0)),
            None
        );
        assert_eq!(
            droid_menu_anchor(layout(0.0), CockpitFaction::Empire, (710.0, 900.0)),
            None
        );
    }

    #[test]
    fn message_alerts_lists_messages_then_the_nine_categories() {
        // FUN_00487a60 adds 0x11c, then 0x171..0x177, 0x179, 0x178; labels
        // are TEXTSTRA 0x311c and 0x3120..0x3128 (word 5 of each record).
        let items = message_alerts_items(|_| 0);
        let labels: Vec<_> = items.iter().map(|item| item.label).collect();
        assert_eq!(
            labels,
            [
                "Messages",
                "Loyalty",
                "Fleets",
                "Mission",
                "Resources",
                "Manufacturing",
                "Defense",
                "Conflict",
                "Chat",
                "Advice"
            ]
        );
        assert_eq!(items[0].command, MessageAlertsCommand::Messages);
        assert!(!items[0].checked);
    }

    #[test]
    fn each_message_alert_reads_and_flips_its_own_category_mask() {
        // FUN_00486fb0 cases 0x171..0x179 and FUN_00487a60 pass these masks
        // to FUN_0048a1c0; they are the Message Index's own (FUN_004697b0).
        let masks: Vec<_> = MESSAGE_ALERT_RECORDS
            .iter()
            .map(|record| (record.command, record.mask))
            .collect();
        assert_eq!(
            masks,
            [
                (0x171, 0x001),
                (0x172, 0x080),
                (0x173, 0x010),
                (0x174, 0x004),
                (0x175, 0x008),
                (0x176, 0x040),
                (0x177, 0x100),
                (0x179, 0x020),
                (0x178, 0x200)
            ]
        );
        for (record, category) in MESSAGE_ALERT_RECORDS
            .iter()
            .zip([0x7a_u16, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82])
        {
            assert_eq!(
                record.mask,
                crate::message_index::category_filter_mask(category),
                "{}",
                record.label
            );
        }
        let items = message_alerts_items(|_| 0);
        for (item, record) in items[1..].iter().zip(MESSAGE_ALERT_RECORDS) {
            assert_eq!(
                item.command,
                MessageAlertsCommand::ToggleAlert { mask: record.mask }
            );
        }
    }

    #[test]
    fn a_message_alert_is_checked_while_its_category_posts_aloud() {
        // FUN_00487a60: `test al, 4; jne` skips the check, so an item is
        // checked while Post Messages Silently (bit 4) is clear.
        let silent_fleets = |mask: u16| {
            if mask == 0x080 {
                crate::message_log::POST_SILENTLY
            } else {
                0
            }
        };
        let items = message_alerts_items(silent_fleets);
        let checked: Vec<_> = items[1..].iter().map(|item| item.checked).collect();
        assert_eq!(
            checked,
            [true, false, true, true, true, true, true, true, true]
        );
        // Other flag bits do not uncheck an item.
        assert!(message_alerts_items(|_| 0xfb)[1..]
            .iter()
            .all(|item| item.checked));
    }

    #[test]
    fn choosing_a_message_alert_flips_only_the_silent_bit() {
        // FUN_00486fb0: FUN_0048a210(mask, FUN_0048a1c0(mask) ^ 4).
        assert_eq!(toggled_alert_flags(0), 4);
        assert_eq!(toggled_alert_flags(4), 0);
        assert_eq!(toggled_alert_flags(0x13), 0x17);
        assert_eq!(toggled_alert_flags(0x17), 0x13);
    }

    #[test]
    fn a_right_click_on_the_agents_droid_opens_the_menu() {
        // FUN_004420b0 hit-tests the panel at the click; the larger droid's
        // aperture (FUN_0042adb0).
        assert!(agent_panel_contains(
            CockpitFaction::Alliance,
            (560.0, 400.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (320.0, 420.0)
        ));
        assert!(agent_panel_contains(CockpitFaction::Empire, (50.0, 400.0)));
        // (541, 337, 67, 116): right and bottom edges are outside.
        assert!(agent_panel_contains(
            CockpitFaction::Alliance,
            (541.0, 337.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (608.0, 400.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (560.0, 453.0)
        ));
        assert!(!agent_panel_contains(
            CockpitFaction::Alliance,
            (540.0, 400.0)
        ));
    }

    /// A right click's release at `screen` on a canvas at (10, 20), scale 2.
    fn right_click(state: &mut AgentMenuState, screen: egui::Pos2, scale: f32) -> bool {
        use crate::cockpit::CockpitViewport;
        let layout = CockpitLayout {
            canvas: CockpitViewport {
                x: 10.0,
                y: 20.0,
                width: 640.0 * scale,
                height: 480.0 * scale,
            },
            galaxy: CockpitViewport {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            scale,
        };
        let ctx = egui::Context::default();
        let mut opened = false;
        for pressed in [true, false] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1000.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(screen),
                    egui::Event::PointerButton {
                        pos: screen,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                opened |=
                    open_agent_menu_on_right_click(ctx, state, layout, CockpitFaction::Alliance);
            });
        }
        opened
    }

    #[test]
    fn a_right_button_release_on_the_droid_anchors_the_menu_at_its_canvas_point() {
        // FUN_00422ce0 WM_RBUTTONUP -> FUN_004420b0 at the release point.
        let mut state = AgentMenuState::default();
        assert!(right_click(
            &mut state,
            egui::pos2(10.0 + 560.0 * 2.0, 20.0 + 400.0 * 2.0),
            2.0
        ));
        assert_eq!(state.anchor, Some((560.0, 400.0)));

        let mut state = AgentMenuState::default();
        assert!(!right_click(
            &mut state,
            egui::pos2(10.0 + 320.0 * 2.0, 20.0 + 420.0 * 2.0),
            2.0
        ));
        assert_eq!(state.anchor, None);

        let mut state = AgentMenuState::default();
        assert!(!right_click(&mut state, egui::pos2(570.0, 420.0), 0.0));
        assert_eq!(state.anchor, None);
    }
}
