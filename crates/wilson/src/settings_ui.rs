// SPDX-License-Identifier: GPL-3.0-or-later
//! The floating Custom dialog and portable configuration panel.
use crate::config::{Config, TestGroup, TestHoliday, TestTime, TidePreference, Transition};
use crate::scale::{Filter, ScaleMode};
use eframe::egui;
use egui::viewport::WindowLevel;
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use wilson_engine::{story::LOWTIDE_OK, DayNight, STORY_SCENES};

struct SceneCatalogEntry {
    short_name: &'static str,
    description: &'static str,
}

#[derive(Clone, Copy)]
struct PlaybackStatus {
    day: u8,
    scene: u8,
}

// Reviewed names and descriptions from the project scene catalog. Keeping this
// beside the selector means the Windows and Raspberry Pi interfaces use the
// same catalog, rather than two lists which inevitably drift apart.
const SCENE_CATALOG: [SceneCatalogEntry; 63] = [
    SceneCatalogEntry { short_name: "Dive Scorecards", description: "Johnny makes three dives from the palm tree: feet-first, belly flop, and cannonball. Sea creatures hold up scorecards." },
    SceneCatalogEntry { short_name: "Gull Nest on Head", description: "A gull lands on Johnny's head and starts a nest. Johnny misses it with a club and hits himself." },
    SceneCatalogEntry { short_name: "Gull Steals Clothes", description: "While Johnny bathes, a gull steals his clothes and carries them to the palm tree." },
    SceneCatalogEntry { short_name: "Gull Reads Book", description: "A gull steals Johnny's book and reads it from the top of the palm tree." },
    SceneCatalogEntry { short_name: "Palm Tree Dive", description: "Johnny performs an ordinary dive from the palm tree." },
    SceneCatalogEntry { short_name: "Rain Dance Lightning", description: "Johnny changes into native dress, rain-dances, and is struck by lightning." },
    SceneCatalogEntry { short_name: "Coconut Wake-Up", description: "Johnny falls asleep reading and wakes when a coconut lands on his head." },
    SceneCatalogEntry { short_name: "Upside-Down Book", description: "Johnny realizes he is reading upside down and turns his book around." },
    SceneCatalogEntry { short_name: "Bathing Viewer Gag", description: "After bathing, Johnny notices the viewer, dresses behind the tree, and shakes his fist." },
    SceneCatalogEntry { short_name: "Sightseeing Boat Dance", description: "Johnny dances for a sightseeing boat, strips naked, and waves leaves at the passengers." },
    SceneCatalogEntry { short_name: "Sandcastle Meltdown", description: "Johnny builds a sandcastle, watches it collapse, and stomps on it." },
    SceneCatalogEntry { short_name: "Lilliputian Tie-Down", description: "Lilliputian pirates arrive by galleon, tie sleeping Johnny down, and a gull nests on him." },
    SceneCatalogEntry { short_name: "Sleeping Roll", description: "Johnny lies down, rolls around in his sleep, and wakes up." },
    SceneCatalogEntry { short_name: "Lilliputian Castle Attack", description: "Tiny pirates attack while Johnny builds a sandcastle, occupy it, and launch tiny fighter planes." },
    SceneCatalogEntry { short_name: "Fire Building Success", description: "After several attempts, Johnny successfully builds a fire and relaxes." },
    SceneCatalogEntry { short_name: "Campfire Cooking", description: "Johnny cooks an octopus, fish, or boot over a fire, with varying bad results." },
    SceneCatalogEntry { short_name: "Night Lilliputian Tie-Down", description: "At night, Lilliputian pirates arrive and tie sleeping Johnny down on the beach." },
    SceneCatalogEntry { short_name: "Fishing Junk Catch", description: "Johnny catches a starfish, a nose-clamping crab, or a boot and throws it back." },
    SceneCatalogEntry { short_name: "Fishing Keeper Catch", description: "Johnny catches wood or a large green fish and hides the catch behind the palm tree." },
    SceneCatalogEntry { short_name: "Giant Octopus Catch", description: "Johnny hooks a giant octopus, which steals his fish and sends him climbing the tree." },
    SceneCatalogEntry { short_name: "Shark Water-Ski", description: "A shark pulls Johnny from shore and tows him around the water on bare feet." },
    SceneCatalogEntry { short_name: "Shark Spits Johnny", description: "A shark swallows Johnny, grimaces at the taste, and spits him back onto shore." },
    SceneCatalogEntry { short_name: "Fish Face Spray", description: "A large green fish stares at Johnny, sprays water in his face, and gets thrown back." },
    SceneCatalogEntry { short_name: "East-Side Junk Catch", description: "Fishing from the east side, Johnny catches unwanted items and throws them back." },
    SceneCatalogEntry { short_name: "East-Side Keeper Catch", description: "Fishing from the east side, Johnny catches a boot or large green fish and hides it." },
    SceneCatalogEntry { short_name: "Return by Plane", description: "Day 11: Johnny returns to the island by plane and parachute at sunset." },
    SceneCatalogEntry { short_name: "SOS Bottle Dream", description: "Day 2: Johnny writes an SOS message, puts it in a bottle, and throws it into the sea." },
    SceneCatalogEntry { short_name: "Message to Suzy", description: "Day 6: Johnny writes a message asking his dream woman to meet him, then bottles it." },
    SceneCatalogEntry { short_name: "SOS Bottle Returns", description: "Johnny finds his own SOS bottle and throws it back into the sea." },
    SceneCatalogEntry { short_name: "SOS Bottle Toss", description: "Johnny writes an SOS message, bottles it, and throws it into the sea." },
    SceneCatalogEntry { short_name: "Office Island Dream", description: "Day 10: Johnny works in an office, dreams of the island and Mary, and regrets leaving." },
    SceneCatalogEntry { short_name: "Mermaid Dinner Date", description: "Day 5: Johnny and Mary share dinner and dance before she swims away." },
    SceneCatalogEntry { short_name: "Mermaid Date Request", description: "Day 4: Johnny asks Mary the Mermaid for a date and she agrees." },
    SceneCatalogEntry { short_name: "Mermaid Watches Fishing", description: "Day 1: Mary watches Johnny fish; he mistakes her splash for a fish." },
    SceneCatalogEntry { short_name: "Mermaid Mainland Invitation", description: "Day 7: Johnny invites Mary to the mainland; she cries and swims away." },
    SceneCatalogEntry { short_name: "Raft Farewell", description: "Day 8: Mary and the shark say goodbye as Johnny leaves by raft." },
    SceneCatalogEntry { short_name: "Heat Meltdown", description: "Johnny tries to cool himself, fans himself, and melts into a puddle." },
    SceneCatalogEntry { short_name: "Shark Bath Scare", description: "A shark leaps from the sea while Johnny tests the water and knocks him into the tree." },
    SceneCatalogEntry { short_name: "Idle West Southwest", description: "Johnny idles on the west side, facing southwest." },
    SceneCatalogEntry { short_name: "Idle West West", description: "Johnny idles on the west side, facing west." },
    SceneCatalogEntry { short_name: "Idle West Northwest", description: "Johnny idles on the west side, facing northwest." },
    SceneCatalogEntry { short_name: "Idle South Southwest", description: "Johnny idles near the south side, facing southwest." },
    SceneCatalogEntry { short_name: "Idle South", description: "Johnny idles near the south side, facing south." },
    SceneCatalogEntry { short_name: "Idle South Southeast", description: "Johnny idles near the south side, facing southeast." },
    SceneCatalogEntry { short_name: "Idle East Northeast", description: "Johnny idles on the east side, facing northeast." },
    SceneCatalogEntry { short_name: "Idle East East", description: "Johnny idles on the east side, facing east." },
    SceneCatalogEntry { short_name: "Idle Northeast Northwest", description: "Johnny idles on the northeast side, facing northwest." },
    SceneCatalogEntry { short_name: "Idle Northeast Northeast", description: "Johnny idles on the northeast side, facing northeast." },
    SceneCatalogEntry { short_name: "Idle Northwest", description: "Johnny idles on the northwest side, facing northwest." },
    SceneCatalogEntry { short_name: "Idle Palm Tree", description: "Johnny idles in front of the palm tree, facing south." },
    SceneCatalogEntry { short_name: "West Spyglass", description: "Johnny scans from west to east with a spyglass; an eyeball moves in the lens." },
    SceneCatalogEntry { short_name: "East Spyglass", description: "Johnny scans from west to east with a spyglass; an eyeball blinks in the lens." },
    SceneCatalogEntry { short_name: "Suzy Finds Bottle", description: "Day 3: Suzy finds Johnny's bottle and imagines the castaway who needs help." },
    SceneCatalogEntry { short_name: "Raft Arrival and Kiss", description: "Day 9: Johnny reaches Suzy by raft; they embrace, kiss, and she scolds him." },
    SceneCatalogEntry { short_name: "Missed Rescue Visitors", description: "Johnny looks the wrong way through his spyglass while aircraft or the SS SOS pass behind him." },
    SceneCatalogEntry { short_name: "Cargo Ship Crush", description: "Johnny tries to flag down a cargo ship, then jumps into the water as it plows through the island." },
    SceneCatalogEntry { short_name: "Coconut Bounce", description: "A coconut bounces over Johnny's head and rolls into the sea." },
    SceneCatalogEntry { short_name: "Coconut Head Spin", description: "A coconut circles Johnny and the tree, spinning his head around before he eats it." },
    SceneCatalogEntry { short_name: "Coconut Chase", description: "Johnny chases a fallen coconut, opens it against the tree, and eats and drinks." },
    SceneCatalogEntry { short_name: "Plane Coconut Crash", description: "Johnny throws a coconut at a passing plane, crashes it, and the pilot parachutes down." },
    SceneCatalogEntry { short_name: "Party Boat Return", description: "Johnny boards a party boat, which later returns him drunk and back on the island." },
    SceneCatalogEntry { short_name: "Raft Building Wave", description: "Johnny keeps building his raft, waves toward the viewer, and brushes himself off." },
    SceneCatalogEntry { short_name: "Island Jog", description: "Johnny changes into jogging clothes, circles the island, and stretches against the tree." },
];

/// Built-in reviewed catalog used to seed a new editable `castaway-continuum.ini` file.
pub(crate) fn default_scene_catalog() -> (Vec<String>, Vec<String>) {
    (
        SCENE_CATALOG
            .iter()
            .map(|entry| entry.short_name.to_owned())
            .collect(),
        SCENE_CATALOG
            .iter()
            .map(|entry| entry.description.to_owned())
            .collect(),
    )
}

pub(crate) fn scene_catalog_text_for_program<'a>(
    cfg: &'a Config,
    ads_name: &str,
    ads_tag: u16,
) -> Option<(u8, &'a str, &'a str)> {
    STORY_SCENES
        .iter()
        .position(|scene| scene.ads_name == ads_name && scene.ads_tag == ads_tag)
        .and_then(|index| {
            cfg.scene_catalog_text((index + 1) as u8)
                .map(|(name, description)| ((index + 1) as u8, name, description))
        })
}

fn preview(cfg: &Config, save_first: bool) -> Result<(), String> {
    if save_first {
        cfg.save()?;
    }
    let socket = UdpSocket::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| error.to_string())?;
    // Preview carries every current setting, not merely the scene selection.
    let request = format!("wilson-preview-config:{}", cfg.preview_wire());
    socket
        .send_to(request.as_bytes(), "127.0.0.1:42781")
        .map_err(|error| error.to_string())?;
    let mut acknowledgement = [0_u8; 64];
    let acknowledged = socket
        .recv_from(&mut acknowledgement)
        .ok()
        .and_then(|(len, _)| std::str::from_utf8(&acknowledgement[..len]).ok())
        .is_some_and(|reply| reply == "ok");
    acknowledged
        .then_some(())
        .ok_or_else(|| "No running Wilson window answered the preview request.".to_owned())
}

/// Ask the running display which story day it is actually playing. The Custom
/// dialog is a separate process, so it must not guess from its last button press.
fn running_playback_status() -> Option<PlaybackStatus> {
    let socket = UdpSocket::bind("127.0.0.1:0").ok()?;
    socket
        .set_read_timeout(Some(Duration::from_millis(80)))
        .ok()?;
    socket.send_to(b"wilson-status", "127.0.0.1:42781").ok()?;
    let mut response = [0_u8; 32];
    let (len, _) = socket.recv_from(&mut response).ok()?;
    let response = std::str::from_utf8(&response[..len]).ok()?;
    let (day, scene) = response.strip_prefix("day:")?.split_once(";scene:")?;
    let day = day.parse::<u8>().ok()?;
    let scene = scene.parse::<u8>().ok()?;
    ((1..=11).contains(&day) && (1..=63).contains(&scene)).then_some(PlaybackStatus { day, scene })
}

fn scene_label(cfg: &Config, index: usize, ads: &str, tag: u16, day: u8) -> String {
    if let Some((short_name, _)) = cfg.scene_catalog_text(index as u8) {
        return format!("{index:02}  {}  ({ads} #{tag}, day {day})", short_name);
    }
    let verified = match (ads, tag, day) {
        ("MARY.ADS", 2, 1) => Some("Day 1 — Mermaid watches Johnny fishing"),
        ("JOHNNY.ADS", 2, 2) => Some("Day 2 — Johnny imagines himself on the island"),
        ("SUZY.ADS", 1, 3) => Some("Day 3 — Suzy receives Johnny's bottle"),
        ("MARY.ADS", 3, 4) => Some("Day 4 — Johnny asks Mermaid for a date"),
        ("MARY.ADS", 1, 5) => Some("Day 5 — Dinner and dance with Mermaid"),
        ("JOHNNY.ADS", 3, 6) => Some("Day 6 — Johnny draws himself with Suzy"),
        ("MARY.ADS", 4, 7) => Some("Day 7 — Johnny invites Mermaid to mainland"),
        ("MARY.ADS", 5, 8) => Some("Day 8 — Johnny says goodbye and paddles away"),
        ("SUZY.ADS", 2, 9) => Some("Day 9 — Johnny reaches mainland and meets Suzy"),
        ("JOHNNY.ADS", 6, 10) => Some("Day 10 — Office dream of the island"),
        ("JOHNNY.ADS", 1, 11) => Some("Day 11 — Johnny returns by plane"),
        _ => None,
    };
    let text = verified.map_or_else(|| format!("{ads} #{tag}"), str::to_owned);
    format!("{index:02}  {text}")
}

pub fn open(config: Config) {
    let manager_shell = std::env::var_os("CASTAWAY_CONTINUUM_CUSTOM_DIALOG").is_some();
    let island_location_bar = std::env::var_os("CASTAWAY_CONTINUUM_ISLAND_LOCATION_BAR").is_some();
    let tab = std::env::var("CASTAWAY_CONTINUUM_SETTINGS_TAB")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map(|tab| tab.min(4))
        .unwrap_or(0);
    // Custom must fit a 7-inch 800×480 touchscreen as well as a Windows desktop.
    let (inner_size, min_size) = if island_location_bar {
        // Five fixed-width controls plus their normal gaps and panel margins:
        // exactly one content row, with no dead white space underneath.
        // The central panel keeps a small top/bottom margin; 34px left the
        // 26px buttons partly outside the client area on Windows.
        // Keep the bottom gap below this row equal to the Scene Selection
        // navigation strip rather than leaving an oversized blank band.
        ([408.0, 48.0], [408.0, 48.0])
    } else if manager_shell {
        // One mouse/touch-friendly Custom Settings window. It retains the
        // compact scene card but also hosts the environment tabs. Its width
        // is the seven 72px Scene buttons, normal gaps, and a small margin.
        ([568.0, 300.0], [568.0, 240.0])
    } else {
        ([760.0, 620.0], [640.0, 480.0])
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(inner_size)
            .with_min_inner_size(min_size)
            // Keep normal Windows chrome: it provides the requested close and
            // maximize controls instead of making this another borderless overlay.
            .with_decorations(true)
            .with_resizable(!island_location_bar)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_title(if island_location_bar {
                "Island location"
            } else if manager_shell {
                "Custom Settings"
            } else {
                "Custom"
            }),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Custom",
        options,
        Box::new(|_| {
            // The Classic Scene Set uses TestGroup::Off: it deliberately
            // leaves the original screensaver's scene scheduler in charge.
            let scene_group = match config.test_group {
                TestGroup::Off => CLASSIC_SCENE_SET,
                TestGroup::All | TestGroup::Normal => 0,
                TestGroup::Special => 2,
                TestGroup::Visitors => 3,
                TestGroup::Common => 4,
                TestGroup::Uncommon => 5,
                TestGroup::Favorites => FAVORITES_SCENE_SET,
            };
            Ok(Box::new(App {
                cfg: config,
                tab,
                saved: false,
                manager_shell,
                island_location_bar,
                scene_filter: String::new(),
                scene_help: None,
                scene_browser: 1,
                scene_group,
                running_special_day: None,
                running_scene: None,
                last_status_poll: Instant::now() - Duration::from_secs(1),
                preview_error: None,
                preview_status: None,
            }))
        }),
    );
}

struct App {
    cfg: Config,
    tab: usize,
    saved: bool,
    manager_shell: bool,
    island_location_bar: bool,
    scene_filter: String,
    scene_help: Option<u8>,
    scene_browser: u8,
    scene_group: usize,
    running_special_day: Option<u8>,
    running_scene: Option<u8>,
    last_status_poll: Instant,
    preview_error: Option<String>,
    preview_status: Option<String>,
}

const ISLAND_STEP_PIXELS: i32 = 5;
/// The first Scene Set entry: preserve the screensaver's original scene order.
const CLASSIC_SCENE_SET: usize = 6;
/// The user's own persistent scene collection.
const FAVORITES_SCENE_SET: usize = 7;

fn clamp_island_offset((x, y): (i32, i32)) -> (i32, i32) {
    (x.clamp(-640, 640), y.clamp(-480, 480))
}

fn nudged_island_offset((x, y): (i32, i32), delta: (i32, i32)) -> (i32, i32) {
    clamp_island_offset((x.saturating_add(delta.0), y.saturating_add(delta.1)))
}

impl App {
    fn set_island_offset(&mut self, offset: (i32, i32)) {
        let (x, y) = clamp_island_offset(offset);
        self.cfg.test_offset_enabled = true;
        self.cfg.test_offset_x = x;
        self.cfg.test_offset_y = y;
    }

    fn nudge_island_offset(&mut self, delta: (i32, i32)) {
        self.set_island_offset(nudged_island_offset(
            (self.cfg.test_offset_x, self.cfg.test_offset_y),
            delta,
        ));
    }

    fn apply_preview_with_save(&mut self, save_first: bool) {
        // Changing any Custom control deliberately re-enables the saved Custom
        // startup profile; Original only disables application of those overrides.
        self.cfg.custom_enabled = true;
        match preview(&self.cfg, save_first) {
            Ok(()) => {
                self.saved = true;
                self.preview_error = None;
                self.preview_status = Some("Applied and saved.".to_owned());
            }
            Err(error) => {
                self.preview_status = None;
                self.preview_error = Some(error);
            }
        }
    }

    fn reload_saved_island_location(&mut self) {
        // The floating controllers are separate processes. The Display menu
        // may have changed this checkbox after this window was opened, so do
        // not allow a stale scene dialog to write the old value back.
        let saved = Config::load();
        self.cfg.test_offset_enabled = saved.test_offset_enabled;
        self.cfg.test_offset_x = saved.test_offset_x;
        self.cfg.test_offset_y = saved.test_offset_y;
    }

    fn apply_preview(&mut self) {
        self.reload_saved_island_location();
        self.apply_preview_with_save(true);
    }

    fn save_island_location(&mut self) {
        self.cfg.test_offset_enabled = true;
        self.apply_preview_with_save(true);
    }

    fn apply_preview_unsaved(&mut self) {
        self.apply_preview_with_save(false);
        self.preview_status = Some("Applied for this session; press Save to keep it.".to_owned());
    }

    fn island_location_controller(&mut self, ctx: &egui::Context) {
        ctx.style_mut(|style| {
            // The controls themselves are explicitly 26px high. Leaving the
            // global minimum at 42px silently enlarged them and clipped this
            // fixed-height utility window.
            style.spacing.interact_size.y = 26.0;
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.island_location_controls(ui);
            // The scene selector has an explicit 8px lower inset beneath its
            // navigation row; keep this companion strip visually identical.
            ui.add_space(8.0);
        });
    }

    fn island_location_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let mut moved = false;
            for (label, dx, dy) in [
                ("Left", -ISLAND_STEP_PIXELS, 0),
                ("Right", ISLAND_STEP_PIXELS, 0),
                ("Up", 0, -ISLAND_STEP_PIXELS),
                ("Down", 0, ISLAND_STEP_PIXELS),
            ] {
                if ui
                    .add_sized([72.0, 26.0], egui::Button::new(label))
                    .clicked()
                {
                    self.nudge_island_offset((dx, dy));
                    moved = true;
                }
            }
            if ui
                .add_sized([72.0, 26.0], egui::Button::new("Save"))
                .clicked()
            {
                self.save_island_location();
            } else if moved {
                self.apply_preview_unsaved();
            }
        });
    }

    fn scene_group_numbers(&self) -> Vec<u8> {
        if self.scene_group == FAVORITES_SCENE_SET {
            return self.cfg.favorite_scenes.clone();
        }
        if self.scene_group == 2 {
            // Story scenes are stored by ADS file, not story chronology. The
            // Special browser must therefore use Day 1 through Day 11 order.
            return (1..=11).map(Self::special_scene_for_day).collect();
        }
        STORY_SCENES
            .iter()
            .enumerate()
            .filter(|(_, scene)| match self.scene_group {
                CLASSIC_SCENE_SET => true,
                0 => true,
                1 => scene.day == 0,
                2 => unreachable!("special scenes are returned above in story-day order"),
                3 => scene.ads_name == "VISITOR.ADS",
                4 => {
                    scene.day == 0
                        && matches!(
                            scene.ads_name,
                            "ACTIVITY.ADS" | "BUILDING.ADS" | "STAND.ADS"
                        )
                }
                _ => {
                    scene.day == 0
                        && matches!(
                            scene.ads_name,
                            "FISHING.ADS" | "MISCGAG.ADS" | "VISITOR.ADS" | "WALKSTUF.ADS"
                        )
                }
            })
            .map(|(index, _)| (index + 1) as u8)
            .collect()
    }

    fn move_scene_browser(&mut self, amount: isize, edge: bool) {
        let scenes = self.scene_group_numbers();
        if scenes.is_empty() {
            return;
        }
        if edge {
            self.scene_browser = if amount < 0 {
                scenes[0]
            } else {
                *scenes.last().unwrap()
            };
            return;
        }
        let current = scenes
            .iter()
            .position(|number| *number == self.scene_browser)
            .unwrap_or(0);
        let next = (current as isize + amount).rem_euclid(scenes.len() as isize) as usize;
        self.scene_browser = scenes[next];
    }

    fn selected_scene_group(&self) -> TestGroup {
        match self.scene_group {
            CLASSIC_SCENE_SET => TestGroup::Off,
            0 => TestGroup::All,
            // "Normal" was an early diagnostic slice of the catalog. It is
            // deliberately no longer exposed in the user-facing selector;
            // legacy saved settings open as All 63 instead.
            1 => TestGroup::All,
            2 => TestGroup::Special,
            3 => TestGroup::Visitors,
            4 => TestGroup::Common,
            5 => TestGroup::Uncommon,
            _ => TestGroup::Favorites,
        }
    }

    fn special_scene_for_day(day: u8) -> u8 {
        STORY_SCENES
            .iter()
            .position(|scene| scene.day == day)
            .map(|index| (index + 1) as u8)
            .expect("every special day has one verified catalog scene")
    }

    /// Start sequential playback at the browser scene. Repeating is deliberately
    /// separate: browsing a group must not silently turn Repeat on.
    fn start_group_at_browser_scene(&mut self) {
        self.cfg.test_scene = 0;
        self.cfg.test_group = self.selected_scene_group();
        self.cfg.test_group_start = self.scene_browser;
        self.cfg.special_cycle = self.cfg.test_group == TestGroup::Special;
        self.running_scene = Some(self.scene_browser);
        if self.cfg.special_cycle {
            // An empty special-day list means "start at day 1" in the engine.
            // The catalog needs all days explicitly so it can begin at the scene
            // the user actually selected and then proceed in order.
            self.cfg.special_days = (1..=11).collect();
            self.running_special_day = Some(STORY_SCENES[usize::from(self.scene_browser - 1)].day);
        } else if self.cfg.test_group != TestGroup::Favorites {
            self.running_special_day = None;
        }
    }

    fn displayed_scene_number(&self) -> u8 {
        if self.cfg.test_scene == 0 {
            return self.running_scene.unwrap_or(self.scene_browser);
        }
        self.scene_browser
    }

    fn refresh_running_playback_status(&mut self) {
        if !self.manager_shell || self.cfg.test_scene != 0 {
            return;
        }
        if self.last_status_poll.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.last_status_poll = Instant::now();
        if let Some(status) = running_playback_status() {
            self.running_special_day = Some(status.day);
            self.running_scene = Some(status.scene);
        }
    }

    fn group_sequence_label(&self) -> Option<String> {
        if self.cfg.test_scene != 0 || self.cfg.test_group == TestGroup::Off {
            return None;
        }
        if self.scene_group == 2 {
            let day = self.running_special_day.unwrap_or_else(|| {
                STORY_SCENES[usize::from(self.displayed_scene_number() - 1)].day
            });
            return Some(format!("Special Day {day} of 11"));
        }
        if self.scene_group == FAVORITES_SCENE_SET {
            let scenes = self.scene_group_numbers();
            let position = scenes
                .iter()
                .position(|&number| number == self.displayed_scene_number())
                .map(|index| index + 1)
                .unwrap_or(1);
            return Some(format!("Favorites {position} of {}", scenes.len()));
        }
        let scenes = self.scene_group_numbers();
        let scene = self.displayed_scene_number();
        let position = scenes
            .iter()
            .position(|&number| number == scene)
            .map(|index| index + 1)
            .unwrap_or(1);
        let name = match self.scene_group {
            0 => "All",
            1 => "All",
            3 => "Visitors",
            4 => "Common",
            _ => "Uncommon",
        };
        Some(format!("{name} {position} of {}", scenes.len()))
    }

    fn scene_card_browser(&mut self, ui: &mut egui::Ui) -> bool {
        let mut playback_changed = false;
        if let Some(first_warning) = self.cfg.catalog_warnings.first() {
            ui.colored_label(
                egui::Color32::from_rgb(155, 75, 20),
                format!(
                    "Scene catalog issue ({}): {first_warning}",
                    self.cfg.catalog_warnings.len()
                ),
            );
            ui.small("Built-in text remains active for incomplete or invalid catalog entries.");
        }
        ui.horizontal(|ui| {
            let selected_group = self.scene_group;
            ui.scope(|ui| {
                ui.spacing_mut().interact_size.y = 26.0;
                egui::ComboBox::from_id_salt("scene-group")
                    .selected_text(match self.scene_group {
                        CLASSIC_SCENE_SET => "OFF",
                        0 => "All",
                        1 => "All",
                        2 => "Special",
                        3 => "Visitors",
                        4 => "Common",
                        5 => "Uncommon",
                        _ => "Favorites",
                    })
                    .width(130.0)
                    // Egui points its stock icon toward the popup. When the
                    // popup must open above the compact selector, that makes
                    // the closed control look like an upside-down dropdown.
                    // A selector arrow should simply point down, full stop.
                    .icon(|ui, rect, visuals, _, _| {
                        let rect = egui::Rect::from_center_size(
                            rect.center(),
                            egui::vec2(rect.width() * 0.7, rect.height() * 0.45),
                        );
                        ui.painter().add(egui::Shape::convex_polygon(
                            vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
                            visuals.fg_stroke.color,
                            egui::Stroke::NONE,
                        ));
                    })
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().interact_size.y = 26.0;
                        for (group, label) in [
                            (CLASSIC_SCENE_SET, "OFF"),
                            (0, "All"),
                            (2, "Special"),
                            (3, "Visitors"),
                            (4, "Common"),
                            (5, "Uncommon"),
                            (FAVORITES_SCENE_SET, "Favorites"),
                        ] {
                            if ui
                                .selectable_value(&mut self.scene_group, group, label)
                                .clicked()
                            {
                                // Egui does not dismiss a ComboBox when the
                                // selected value is unchanged. A second click
                                // should still behave like a finished choice.
                                // ComboBox popups are not menus, so
                                // `close_menu` does nothing here. Close the
                                // actual topmost popup, including when the
                                // chosen value was already selected.
                                ui.memory_mut(|memory| memory.close_popup());
                            }
                        }
                    });
            });
            if let Some(label) = self.group_sequence_label() {
                ui.label(
                    egui::RichText::new(label)
                        .size(14.0)
                        .color(egui::Color32::from_rgb(55, 55, 55)),
                );
            }
            if self.scene_group != selected_group {
                playback_changed = true;
            }
        });
        if playback_changed {
            if self.scene_group == CLASSIC_SCENE_SET {
                self.cfg.test_scene = 0;
                self.cfg.test_group = TestGroup::Off;
                self.cfg.test_group_start = 0;
                self.cfg.special_cycle = false;
                self.cfg.special_days.clear();
                self.running_scene = None;
                self.running_special_day = None;
            } else if self.scene_group == 2 {
                // Special is a story-day sequence, not an arbitrary slice of
                // the catalog: always begin it at Day 1.
                self.scene_browser = Self::special_scene_for_day(1);
            } else {
                let scenes = self.scene_group_numbers();
                // Choosing a group starts its catalog at its first scene.
                if let Some(&first) = scenes.first() {
                    self.scene_browser = first;
                }
            }
            // Classic is intentionally not a group playback mode. Starting a
            // group here would immediately override TestGroup::Off and turn
            // it into a custom all-scenes sequence. Because apparently a
            // setting called Classic needs a special case to remain classic.
            if self.scene_group != CLASSIC_SCENE_SET && !self.scene_group_numbers().is_empty() {
                self.start_group_at_browser_scene();
            }
            playback_changed = true;
        }
        if self.scene_group == CLASSIC_SCENE_SET && self.running_scene.is_none() {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("Original scene schedule")
                    .size(17.0)
                    .color(egui::Color32::from_rgb(20, 20, 20)),
            );
            ui.label(
                egui::RichText::new("Waiting for the running display to report its current scene.")
                    .size(15.0)
                    .color(egui::Color32::from_rgb(20, 20, 20)),
            );
            // Keep the navigation strip locked in place just as it is for a
            // catalog entry, rather than letting this shorter text yank it up.
            ui.allocate_space(ui.available_size());
            return playback_changed;
        }
        if self.scene_group == FAVORITES_SCENE_SET && self.scene_group_numbers().is_empty() {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("No favorite scenes selected yet.")
                    .size(17.0)
                    .color(egui::Color32::from_rgb(20, 20, 20)),
            );
            ui.label(
                egui::RichText::new("Choose a scene in another set, then check Favorite.")
                    .size(15.0)
                    .color(egui::Color32::from_rgb(20, 20, 20)),
            );
            ui.allocate_space(ui.available_size());
            return playback_changed;
        }
        let displayed_scene = self.displayed_scene_number();
        let index = usize::from(displayed_scene - 1);
        let scene = &STORY_SCENES[index];
        let (short_name, description) = self
            .cfg
            .scene_catalog_text(displayed_scene)
            .expect("the editable catalog has 63 default entries");
        ui.add_space(2.0);
        let scene_heading = if self.scene_group == CLASSIC_SCENE_SET {
            format!("Now playing: Scene {:02} - {}", displayed_scene, short_name)
        } else {
            format!("Scene {:02} - {}", displayed_scene, short_name)
        };
        ui.label(
            egui::RichText::new(scene_heading)
                .size(17.0)
                .color(egui::Color32::from_rgb(20, 20, 20)),
        );
        ui.label(
            egui::RichText::new(description)
                .size(15.0)
                .color(egui::Color32::from_rgb(20, 20, 20)),
        );
        let mut is_favorite = self.cfg.favorite_scenes.contains(&displayed_scene);
        if ui.checkbox(&mut is_favorite, "Favorite").changed() {
            if is_favorite {
                self.cfg.favorite_scenes.push(displayed_scene);
                self.cfg.favorite_scenes.sort_unstable();
                self.cfg.favorite_scenes.dedup();
            } else {
                self.cfg
                    .favorite_scenes
                    .retain(|&scene| scene != displayed_scene);
                if self.scene_group == FAVORITES_SCENE_SET {
                    let scenes = self.scene_group_numbers();
                    if let Some(&next) = scenes.first() {
                        self.scene_browser = next;
                        self.start_group_at_browser_scene();
                    } else {
                        self.cfg.test_scene = 0;
                        self.cfg.test_group = TestGroup::Favorites;
                        self.cfg.test_group_start = 0;
                    }
                }
            }
            playback_changed = true;
        }
        let (tide_status, tide_color) = if scene.flags & LOWTIDE_OK != 0 {
            (
                "Low tide: available for this scene.",
                egui::Color32::from_rgb(35, 100, 50),
            )
        } else {
            (
                "Low tide: not available for this scene.",
                egui::Color32::from_rgb(120, 65, 35),
            )
        };
        // Reserve the remaining card area. Bottom-up layout makes the tide
        // availability/fallback line stay locked to the dialog's lower-left
        // corner, with the technical filename immediately above it.
        let footer_area = ui.available_size();
        ui.allocate_ui_with_layout(
            footer_area,
            egui::Layout::bottom_up(egui::Align::LEFT),
            |ui| {
                ui.label(
                    egui::RichText::new(tide_status)
                        .size(15.0)
                        .color(tide_color),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Program animation file: {} #{}",
                        scene.ads_name, scene.ads_tag
                    ))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(75, 75, 75)),
                );
            },
        );
        playback_changed
    }

    fn scene_navigation(&mut self, ui: &mut egui::Ui) -> bool {
        let mut browser_changed = false;
        let mut playback_changed = false;
        let repeat_active = self.cfg.test_scene == self.scene_browser;
        let navigation_enabled = self.scene_group != CLASSIC_SCENE_SET
            && !(self.scene_group == FAVORITES_SCENE_SET && self.scene_group_numbers().is_empty());
        // Keep every navigation control on one shared baseline. Wrapped rows
        // caused the first button to sit a pixel higher on some Windows DPI
        // configurations.
        let navigation_size = egui::vec2(ui.available_width(), 26.0);
        ui.allocate_ui_with_layout(
            navigation_size,
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add_enabled_ui(navigation_enabled, |ui| {
                    if ui
                        .add_sized([72.0, 26.0], egui::Button::new("First"))
                        .clicked()
                    {
                        self.move_scene_browser(-1, true);
                        browser_changed = true;
                    }
                    if ui
                        .add_sized([72.0, 26.0], egui::Button::new("-5 Scenes"))
                        .clicked()
                    {
                        self.move_scene_browser(-5, false);
                        browser_changed = true;
                    }
                    if ui
                        .add_sized([72.0, 26.0], egui::Button::new("Previous"))
                        .clicked()
                    {
                        self.move_scene_browser(-1, false);
                        browser_changed = true;
                    }
                    if ui
                        .add_sized([72.0, 26.0], egui::Button::new("Next"))
                        .clicked()
                    {
                        self.move_scene_browser(1, false);
                        browser_changed = true;
                    }
                    if ui
                        .add_sized([72.0, 26.0], egui::Button::new("+5 Scenes"))
                        .clicked()
                    {
                        self.move_scene_browser(5, false);
                        browser_changed = true;
                    }
                    if ui
                        .add_sized([72.0, 26.0], egui::Button::new("Last"))
                        .clicked()
                    {
                        self.move_scene_browser(1, true);
                        browser_changed = true;
                    }
                    if ui
                        .add_sized(
                            [72.0, 26.0],
                            egui::Button::new("Repeat").selected(repeat_active),
                        )
                        .clicked()
                    {
                        if repeat_active {
                            // Resume with the next member of the selected group, not with
                            // normal/random playback and not at the beginning of the list.
                            self.move_scene_browser(1, false);
                            self.cfg.test_scene = 0;
                            self.start_group_at_browser_scene();
                        } else {
                            self.cfg.test_scene = self.scene_browser;
                            self.cfg.test_group = self.selected_scene_group();
                            self.cfg.test_group_start = self.scene_browser;
                            if self.cfg.test_group == TestGroup::Special {
                                self.cfg.special_days = (1..=11).collect();
                            }
                        }
                        self.cfg.special_cycle = self.cfg.test_group == TestGroup::Special;
                        playback_changed = true;
                    }
                });
            },
        );
        if browser_changed {
            self.start_group_at_browser_scene();
            playback_changed = true;
        }
        playback_changed
    }

    fn custom_scenes(&mut self, ui: &mut egui::Ui) {
        if self.manager_shell {
            let playback_changed = self.scene_card_browser(ui);
            if playback_changed {
                self.apply_preview();
            }
            return;
        }
        let mut playback_changed = false;
        ui.heading("Verified scene catalog");
        ui.horizontal(|ui| {
            ui.label("Find");
            ui.text_edit_singleline(&mut self.scene_filter);
            if ui.button("Clear").clicked() {
                self.scene_filter.clear();
            }
        });
        ui.small("Tap a scene to repeat it. Tap ? for its reviewed description.");
        let filter = self.scene_filter.to_ascii_lowercase();
        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, scene) in STORY_SCENES.iter().enumerate() {
                    let number = (i + 1) as u8;
                    let label = scene_label(
                        &self.cfg,
                        number as usize,
                        scene.ads_name,
                        scene.ads_tag,
                        scene.day,
                    );
                    let searchable = format!(
                        "{number} {} {} {} {label}",
                        scene.ads_name, scene.ads_tag, scene.day
                    )
                    .to_ascii_lowercase();
                    if !filter.is_empty() && !searchable.contains(&filter) {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        let list_width = (ui.available_width() - 46.0).max(120.0);
                        if ui
                            .add_sized(
                                [list_width, 42.0],
                                egui::Button::new(label).selected(self.cfg.test_scene == number),
                            )
                            .clicked()
                        {
                            self.cfg.test_scene = number;
                            self.cfg.test_group = TestGroup::All;
                            self.cfg.special_cycle = false;
                            playback_changed = true;
                        }
                        if ui.add_sized([38.0, 42.0], egui::Button::new("?")).clicked() {
                            self.scene_help = Some(number);
                        }
                    });
                }
            });
        if let Some(number) = self.scene_help {
            let index = usize::from(number - 1);
            if let (Some(scene), Some((short_name, description))) =
                (STORY_SCENES.get(index), self.cfg.scene_catalog_text(number))
            {
                let mut open = true;
                egui::Window::new("Scene details")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(true)
                    .default_width(440.0)
                    .show(ui.ctx(), |ui| {
                        ui.heading(format!("Scene {number:02} - {short_name}"));
                        ui.label(format!(
                            "Technical ID: {} #{}",
                            scene.ads_name, scene.ads_tag
                        ));
                        if scene.day != 0 {
                            ui.label(format!("Story day: {}", scene.day));
                        }
                        ui.separator();
                        ui.label(description);
                    });
                if !open {
                    self.scene_help = None;
                }
            } else {
                self.scene_help = None;
            }
        }
        if playback_changed {
            self.apply_preview();
        }
    }

    fn custom(&mut self, ctx: &egui::Context) {
        self.refresh_running_playback_status();
        if self.manager_shell && self.cfg.test_scene == 0 {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        ctx.style_mut(|style| {
            // Keep the bottom navigation strip at the same compact 26px
            // control height as the island-location strip.
            style.spacing.interact_size.y = 26.0;
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
        });
        egui::TopBottomPanel::top("custom-tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (0, "Scenes"),
                    (1, "Island Location"),
                    (2, "Lighting"),
                    (3, "Tide"),
                    (4, "Holiday"),
                ] {
                    ui.selectable_value(&mut self.tab, tab, label);
                }
            });
        });
        if self.tab <= 1 {
            egui::TopBottomPanel::bottom("custom-actions").show(ctx, |ui| {
                ui.add_space(8.0);
                let scene_playback_changed = if self.tab == 0 {
                    self.scene_navigation(ui)
                } else {
                    self.island_location_controls(ui);
                    false
                };
                ui.add_space(8.0);
                if scene_playback_changed {
                    self.apply_preview();
                }
                if let Some(error) = &self.preview_error {
                    ui.colored_label(egui::Color32::RED, format!("Apply failed: {error}"));
                }
            });
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            0 => self.custom_scenes(ui),
            1 => {
                ui.heading("Island location");
                if ui
                    .checkbox(
                        &mut self.cfg.test_offset_enabled,
                        "Use saved island location",
                    )
                    .changed()
                {
                    // This checkbox is the new authoritative island preference.
                    // Do not call `apply_preview()` here: that helper refreshes
                    // island state from disk first for stale companion windows,
                    // which immediately replaced the user's unchecked value.
                    self.apply_preview_with_save(true);
                }
            }
            2 => self.custom_lighting(ui),
            3 => self.custom_tide(ui),
            _ => self.custom_holiday(ui),
        });
    }

    fn custom_lighting(&mut self, ui: &mut egui::Ui) {
        ui.heading("Lighting");
        let mut changed = false;
        if ui
            .radio(
                self.cfg.test_time == TestTime::Any && self.cfg.daynight == DayNight::Original,
                "Classic (8-hour clock)",
            )
            .clicked()
        {
            self.cfg.test_time = TestTime::Any;
            self.cfg.daynight = DayNight::Original;
            changed = true;
        }
        if ui
            .radio(
                self.cfg.test_time == TestTime::Any && self.cfg.daynight == DayNight::Real24h,
                "PC time (24-hour clock)",
            )
            .clicked()
        {
            self.cfg.test_time = TestTime::Any;
            self.cfg.daynight = DayNight::Real24h;
            changed = true;
        }
        changed |= ui
            .radio_value(&mut self.cfg.test_time, TestTime::Day, "Day")
            .changed();
        changed |= ui
            .radio_value(&mut self.cfg.test_time, TestTime::Night, "Night")
            .changed();
        if changed {
            self.apply_preview();
        }
    }

    fn custom_tide(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tide");
        let changed = ui
            .radio_value(
                &mut self.cfg.tide_preference,
                TidePreference::Authentic,
                "Classic",
            )
            .changed()
            || ui
                .radio_value(
                    &mut self.cfg.tide_preference,
                    TidePreference::PreferHigh,
                    "High tide",
                )
                .changed()
            || ui
                .radio_value(
                    &mut self.cfg.tide_preference,
                    TidePreference::PreferLow,
                    "Low tide",
                )
                .changed();
        // Keep the availability caveat fixed at the bottom-left of the tab.
        // It must not wander when the selected radio button changes.
        let footer_area = ui.available_size();
        ui.allocate_ui_with_layout(
            footer_area,
            egui::Layout::bottom_up(egui::Align::LEFT),
            |ui| {
                ui.label(
                    egui::RichText::new(
                        "Low tide falls back to high when the current scene does not allow low tide.",
                    )
                    .size(13.0)
                    .color(egui::Color32::from_rgb(75, 75, 75)),
                );
            },
        );
        if changed {
            self.apply_preview();
        }
    }

    fn custom_holiday(&mut self, ui: &mut egui::Ui) {
        ui.heading("Holiday");
        let mut changed = false;
        for holiday in [
            TestHoliday::Calendar,
            TestHoliday::None,
            TestHoliday::Halloween,
            TestHoliday::StPatrick,
            TestHoliday::Christmas,
            TestHoliday::NewYear,
        ] {
            changed |= ui
                .radio_value(&mut self.cfg.test_holiday, holiday, holiday.label())
                .changed();
        }
        if changed {
            self.apply_preview();
        }
    }

    fn display(&mut self, ui: &mut egui::Ui) {
        ui.heading("Display");
        ui.checkbox(&mut self.cfg.windowed, "Run in a window");
        ui.checkbox(
            &mut self.cfg.full_size_window,
            "Start windowed display full-size",
        );
        ui.checkbox(&mut self.cfg.always_on_top, "Keep window always on top");
        ui.checkbox(
            &mut self.cfg.return_to_primary,
            "Return to primary display if monitor disconnects",
        );
        ui.separator();
        ui.checkbox(&mut self.cfg.mute, "Mute sound effects");
        ui.add(egui::Slider::new(&mut self.cfg.speed, 25..=400).text("Animation speed (%)"));
        egui::ComboBox::from_label("Scale")
            .selected_text(format!("{:?}", self.cfg.scale))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.cfg.scale, ScaleMode::Fit, "Fit");
                ui.selectable_value(&mut self.cfg.scale, ScaleMode::Stretch, "Stretch");
                ui.selectable_value(&mut self.cfg.scale, ScaleMode::Integer, "Integer");
                ui.selectable_value(&mut self.cfg.scale, ScaleMode::Extend, "Extend");
            });
        egui::ComboBox::from_label("Pixel filter")
            .selected_text(format!("{:?}", self.cfg.filter))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.cfg.filter, Filter::Nearest, "Nearest");
                ui.selectable_value(&mut self.cfg.filter, Filter::Linear, "Linear");
                ui.selectable_value(&mut self.cfg.filter, Filter::Xbr, "xBR");
                ui.selectable_value(&mut self.cfg.filter, Filter::Xbrz, "xBRZ");
            });
        ui.checkbox(&mut self.cfg.dedither, "Smooth dithered sea and sky");
    }
    fn playback(&mut self, ui: &mut egui::Ui) {
        ui.heading("Playback");
        ui.checkbox(&mut self.cfg.intro, "Show intro screen");
        ui.add(egui::Slider::new(&mut self.cfg.intro_secs, 1..=60).text("Intro seconds"));
        ui.radio_value(
            &mut self.cfg.daynight,
            DayNight::Original,
            "Original accelerated 8-hour cycle",
        );
        ui.radio_value(
            &mut self.cfg.daynight,
            DayNight::Real24h,
            "Real 24-hour clock",
        );
        ui.radio_value(
            &mut self.cfg.transition,
            Transition::None,
            "Faithful hard cuts",
        );
        ui.radio_value(
            &mut self.cfg.transition,
            Transition::Dissolve,
            "Optional tiled dissolve",
        );
    }
    fn testing(&mut self, ui: &mut egui::Ui) {
        ui.heading("Single Scene Test");
        ui.small("Choose one scene below. It repeats until you choose another one.");
        ui.horizontal(|ui| {
            ui.label("Lighting:");
            ui.radio_value(&mut self.cfg.test_time, TestTime::Day, "Day");
            ui.radio_value(&mut self.cfg.test_time, TestTime::Night, "Night");
        });
        ui.horizontal(|ui| {
            ui.label("Tide preference:");
            ui.radio_value(
                &mut self.cfg.tide_preference,
                TidePreference::Authentic,
                "Authentic/random",
            );
            ui.radio_value(
                &mut self.cfg.tide_preference,
                TidePreference::PreferHigh,
                "Prefer high",
            );
            ui.radio_value(
                &mut self.cfg.tide_preference,
                TidePreference::PreferLow,
                "Prefer low",
            );
        });
        ui.small("Prefer low falls back to high when the selected scene does not allow low tide.");
        egui::ComboBox::from_label("Holiday animation")
            .selected_text(self.cfg.test_holiday.label())
            .show_ui(ui, |ui| {
                for holiday in [
                    TestHoliday::Calendar,
                    TestHoliday::None,
                    TestHoliday::Halloween,
                    TestHoliday::StPatrick,
                    TestHoliday::Christmas,
                    TestHoliday::NewYear,
                ] {
                    ui.selectable_value(&mut self.cfg.test_holiday, holiday, holiday.label());
                }
            });
        ui.checkbox(
            &mut self.cfg.test_offset_enabled,
            "Override island X/Y position",
        );
        ui.add_enabled_ui(self.cfg.test_offset_enabled, |ui| {
            ui.add(
                egui::DragValue::new(&mut self.cfg.test_offset_x)
                    .range(-640..=640)
                    .prefix("X: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.cfg.test_offset_y)
                    .range(-480..=480)
                    .prefix("Y: "),
            );
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.text_edit_singleline(&mut self.scene_filter);
        });
        let selected_label = self
            .cfg
            .test_scene
            .checked_sub(1)
            .and_then(|index| STORY_SCENES.get(index as usize))
            .map(|scene| {
                scene_label(
                    &self.cfg,
                    self.cfg.test_scene as usize,
                    scene.ads_name,
                    scene.ads_tag,
                    scene.day,
                )
            })
            .unwrap_or_else(|| "None selected".to_owned());
        ui.label(format!(
            "Selected scene: {selected_label}   ({} total scenes)",
            STORY_SCENES.len()
        ));
        egui::ScrollArea::vertical()
            // The old 190px cap showed only nine entries and hid the rest of
            // the 63-scene catalog behind an unobvious wheel-scroll area.
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, s) in STORY_SCENES.iter().enumerate() {
                    let n = (i + 1) as u8;
                    let label = scene_label(&self.cfg, n as usize, s.ads_name, s.ads_tag, s.day);
                    let searchable = format!("{n} {} {} {} {label}", s.ads_name, s.ads_tag, s.day)
                        .to_ascii_lowercase();
                    if !self.scene_filter.is_empty()
                        && !searchable.contains(&self.scene_filter.to_ascii_lowercase())
                    {
                        continue;
                    }
                    if ui
                        .selectable_label(self.cfg.test_scene == n, label)
                        .clicked()
                    {
                        self.cfg.test_scene = n;
                    }
                }
            });
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.island_location_bar {
            self.island_location_controller(ctx);
            return;
        }
        if self.manager_shell {
            self.custom(ctx);
            return;
        }
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save configuration").clicked() {
                        match self.cfg.save() {
                            Ok(()) => {
                                self.saved = true;
                                self.preview_error = None;
                                self.preview_status = Some("Saved.".to_owned());
                            }
                            Err(error) => self.preview_error = Some(error),
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save and preview selected scene").clicked() {
                        match preview(&self.cfg, true) {
                            Ok(()) => {
                                self.saved = true;
                                self.preview_error = None;
                                self.preview_status = Some("Saved and preview applied.".to_owned());
                            }
                            Err(error) => self.preview_error = Some(error),
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save configuration and close").clicked() {
                        match self.cfg.save() {
                            Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                            Err(error) => self.preview_error = Some(error),
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit without saving").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                });
            });
        });
        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    match self.cfg.save() {
                        Ok(()) => {
                            self.saved = true;
                            self.preview_error = None;
                            self.preview_status = Some("Saved.".to_owned());
                        }
                        Err(error) => self.preview_error = Some(error),
                    }
                }
                if ui.button("Save and close").clicked() {
                    match self.cfg.save() {
                        Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(error) => self.preview_error = Some(error),
                    }
                }
                if self.tab == 0 && ui.button("Preview (do not save)").clicked() {
                    match preview(&self.cfg, false) {
                        Ok(()) => {
                            self.preview_error = None;
                            self.preview_status =
                                Some("Preview applied — configuration not saved.".to_owned());
                        }
                        Err(error) => self.preview_error = Some(error),
                    }
                }
                if ui.button("Cancel").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if self.saved {
                    ui.label("Saved to castaway-continuum.ini");
                }
                if let Some(status) = &self.preview_status {
                    ui.label(status);
                }
                if let Some(error) = &self.preview_error {
                    ui.colored_label(egui::Color32::RED, format!("Preview failed: {error}"));
                }
            })
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            0 => self.testing(ui),
            1 => self.display(ui),
            2 => self.playback(ui),
            _ => {
                ui.heading("Castaway Continuum");
                ui.label("Settings are portable across Windows and Raspberry Pi/Linux.");
                ui.separator();
                ui.label("Shortcuts while Wilson is focused:");
                ui.monospace("Ctrl+Alt+J        Open Settings");
                ui.monospace("Ctrl+Alt+Shift+J  Exit persistent-display mode");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_arrows_move_exactly_five_pixels_and_stay_in_range() {
        assert_eq!(
            nudged_island_offset((-90, 5), (-ISLAND_STEP_PIXELS, ISLAND_STEP_PIXELS)),
            (-95, 10)
        );
        assert_eq!(nudged_island_offset((-640, -480), (-5, -5)), (-640, -480));
    }
}
