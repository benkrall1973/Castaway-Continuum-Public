// SPDX-License-Identifier: GPL-3.0-or-later
//! User configuration: runtime options read from a small text file (and overridable by
//! CLI flags), stored next to the day state in the per-user state directory.
//!
//! All options have sane defaults, so a missing/partial/unreadable file is fine — the
//! app just runs with defaults. CLI flags (`--windowed`, `--mute`, `--speed`, `--scale`)
//! win over the file but are not persisted, so they stay one-off.

use serde_json::Value;
use std::path::{Path, PathBuf};

use wilson_engine::DayNight;

use crate::scale::{Filter, ScaleMode};

/// Minimum/maximum playback speed (percent of the original timing).
const SPEED_MIN: u32 = 25;
const SPEED_MAX: u32 = 400;

/// The story arc is 11 days; `0` means "auto" (resume/today).
const MAX_STORY_DAY: u8 = 11;
/// Bounds for the story-mode per-day cadence, in real seconds.
const STORY_SECS_MIN: u32 = 5;
const STORY_SECS_MAX: u32 = 86_400;
/// Bounds for the intro-screen hold, in real seconds (`--intro-secs`).
const INTRO_SECS_MIN: u32 = 1;
const INTRO_SECS_MAX: u32 = 30;
/// Default intro hold, in seconds (the original has no intro timer — KB10 §10.2).
const INTRO_SECS_DEFAULT: u32 = 3;
/// The reviewed catalog has one entry for every source-table scene.
const SCENE_CATALOG_COUNT: usize = 63;

/// Scene-transition style (between story runs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Transition {
    /// Hard cut — the faithful original behaviour (the original ships its dissolve disabled).
    #[default]
    None,
    /// The original's dormant LFSR tiled dissolve, resurrected (opt-in; KB10 §10.2).
    Dissolve,
}

impl Transition {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "none" | "cut" | "hard" | "off" => Some(Transition::None),
            "dissolve" | "lfsr" | "tiled" => Some(Transition::Dissolve),
            _ => None,
        }
    }

    /// The canonical name (round-trips with [`Transition::parse`]).
    fn as_str(self) -> &'static str {
        match self {
            Transition::None => "none",
            Transition::Dissolve => "dissolve",
        }
    }
}

/// Diagnostic playback population. `Off` preserves authentic scheduling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TestGroup {
    #[default]
    Off,
    Normal,
    Special,
    Visitors,
    Common,
    Uncommon,
    All,
    Favorites,
}

impl TestGroup {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "off" => Self::Off,
            "normal" => Self::Normal,
            "special" => Self::Special,
            "visitors" | "visitor" => Self::Visitors,
            "common" => Self::Common,
            "uncommon" => Self::Uncommon,
            "all" => Self::All,
            "favorites" | "favourites" => Self::Favorites,
            _ => return None,
        })
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Normal => "normal",
            Self::Special => "special",
            Self::Visitors => "visitors",
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::All => "all",
            Self::Favorites => "favorites",
        }
    }
}

/// Fixed lighting used only by deterministic diagnostic playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TestTime {
    #[default]
    Any,
    Day,
    Night,
}

/// Optional testing preference for the shoreline state. The original chooses low
/// tide randomly only for scenes marked as compatible with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TidePreference {
    /// Preserve the original scene-by-scene random choice.
    #[default]
    Authentic,
    /// Use high tide wherever the scene permits it.
    PreferHigh,
    /// Use low tide when the selected scene permits it; otherwise use high tide.
    PreferLow,
}

/// Holiday prop selection for deterministic scene testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TestHoliday {
    /// Use the current calendar date, as the original does.
    #[default]
    Calendar,
    /// Explicitly suppress holiday props.
    None,
    Halloween,
    StPatrick,
    Christmas,
    NewYear,
}

impl TestHoliday {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "calendar" | "auto" => Self::Calendar,
            "none" | "off" => Self::None,
            "halloween" => Self::Halloween,
            "st_patrick" | "stpatrick" | "patrick" => Self::StPatrick,
            "christmas" => Self::Christmas,
            "new_year" | "newyear" => Self::NewYear,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Calendar => "calendar",
            Self::None => "none",
            Self::Halloween => "halloween",
            Self::StPatrick => "st_patrick",
            Self::Christmas => "christmas",
            Self::NewYear => "new_year",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Calendar => "Calendar default",
            Self::None => "No holiday",
            Self::Halloween => "Halloween",
            Self::StPatrick => "St. Patrick's Day",
            Self::Christmas => "Christmas",
            Self::NewYear => "New Year's",
        }
    }
}

impl TidePreference {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "authentic" | "random" | "auto" => Self::Authentic,
            "high" | "prefer_high" => Self::PreferHigh,
            "low" | "prefer_low" => Self::PreferLow,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Authentic => "authentic",
            Self::PreferHigh => "prefer_high",
            Self::PreferLow => "prefer_low",
        }
    }
}

impl TestTime {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "any" | "auto" => Self::Any,
            "day" => Self::Day,
            "night" => Self::Night,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Day => "day",
            Self::Night => "night",
        }
    }
}

/// Where the optional scene-information panel sits on the display.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SceneInfoCorner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl SceneInfoCorner {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "top_left" | "topleft" => Self::TopLeft,
            "top_right" | "topright" => Self::TopRight,
            "bottom_left" | "bottomleft" => Self::BottomLeft,
            "bottom_right" | "bottomright" => Self::BottomRight,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
        }
    }
}

/// Strength of the optional post-render CRT scanline effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CtrScanlines {
    #[default]
    Off,
    Light,
    Medium,
    Strong,
}

impl CtrScanlines {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "off" | "none" => Self::Off,
            "light" => Self::Light,
            "medium" => Self::Medium,
            "strong" => Self::Strong,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Strong => "strong",
        }
    }
}

/// Amount of geometric warp used to mimic the face of a curved CRT tube.
/// This is deliberately separate from scanlines: a user may like either
/// effect on its own, or combine both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CrtCurvature {
    #[default]
    Off,
    Gentle,
    Curved,
    Strong,
}

impl CrtCurvature {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "off" | "none" => Self::Off,
            "gentle" | "light" => Self::Gentle,
            "curved" | "medium" => Self::Curved,
            "strong" => Self::Strong,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Gentle => "gentle",
            Self::Curved => "curved",
            Self::Strong => "strong",
        }
    }
}

/// Strength of the optional RGB phosphor grille overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CrtPhosphorMask {
    #[default]
    Off,
    Subtle,
    Visible,
}

impl CrtPhosphorMask {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "off" | "none" => Self::Off,
            "subtle" | "light" => Self::Subtle,
            "visible" | "strong" => Self::Visible,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Subtle => "subtle",
            Self::Visible => "visible",
        }
    }
}

/// Optional bright-pixel glow that mimics light scattering in CRT glass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CrtBloom {
    #[default]
    Off,
    Subtle,
}

impl CrtBloom {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "off" | "none" => Self::Off,
            "subtle" | "light" | "on" => Self::Subtle,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Subtle => "subtle",
        }
    }
}

/// Overall colour response of the virtual CRT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CrtColorProfile {
    #[default]
    Neutral,
    Classic,
    Warm,
}

impl CrtColorProfile {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "neutral" | "off" | "original" => Self::Neutral,
            "classic" | "crt" => Self::Classic,
            "warm" | "warm_tv" | "tv" => Self::Warm,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Classic => "classic",
            Self::Warm => "warm",
        }
    }
}

/// Horizontal-only beam blending for a CRT-like pixel response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CrtHorizontalSoftness {
    #[default]
    Off,
    Light,
    Classic,
}

impl CrtHorizontalSoftness {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "off" | "none" => Self::Off,
            "light" | "subtle" => Self::Light,
            "classic" | "strong" => Self::Classic,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Light => "light",
            Self::Classic => "classic",
        }
    }
}

/// The app's runtime options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Run in a 640×480 window instead of fullscreen (handy for development).
    pub windowed: bool,
    /// Start a windowed display maximized instead of at its normal-size window.
    pub full_size_window: bool,
    /// Keep a windowed display above ordinary application windows.
    pub always_on_top: bool,
    /// Legacy setting retained for older configuration files.
    pub ignore_mouse_close: bool,
    /// Return the window to the primary display when its display disappears.
    pub return_to_primary: bool,
    /// Disable sound effects.
    pub mute: bool,
    /// Playback speed, percent of the original timing (100 = original), clamped.
    pub speed: u32,
    /// How the frame is scaled into the window.
    pub scale: ScaleMode,
    /// How upscaled pixels are sampled (nearest = crisp/retro, linear = smooth,
    /// xbr = edge-directed "HD" (dissolves dither), xbrz = edge-directed, keeps dither).
    pub filter: Filter,
    /// Smooth the dithered backgrounds (sea/sky) before scaling — off by default, since
    /// the dithering is the authentic 1992 look.
    pub dedither: bool,
    /// Show date-based Halloween, St. Patrick's Day, Christmas, and New Year props.
    pub holiday_animations: bool,
    /// How the day/night cycle is driven (original 8-hour vs real 24-hour).
    pub daynight: DayNight,
    /// Show diagnostics: a per-second status line on stdout and an on-screen HUD overlay.
    pub debug: bool,
    /// Show the current scene's friendly title and description on the display.
    pub scene_info_overlay: bool,
    /// Selected corner for the scene-information panel.
    pub scene_info_corner: SceneInfoCorner,
    /// Include the catalog's long description below the scene number and name.
    pub scene_info_long_description: bool,
    /// Optional CRT scanline strength, applied after all scene rendering.
    pub crt_scanlines: CtrScanlines,
    /// Optional barrel distortion and rounded-corner effect for a CRT tube.
    pub crt_curvature: CrtCurvature,
    /// Optional RGB phosphor grille over the rendered scene.
    pub crt_phosphor_mask: CrtPhosphorMask,
    /// Optional restrained bloom around bright pixels.
    pub crt_bloom: CrtBloom,
    /// Overall CRT colour response.
    pub crt_color_profile: CrtColorProfile,
    /// Horizontal-only CRT beam softness.
    pub crt_horizontal_softness: CrtHorizontalSoftness,
    /// Show the original's intro screen (`INTRO.SCR`) once at startup (default on, like the
    /// original's `Introduction` option). Disable with `--no-intro`.
    pub intro: bool,
    /// How long the intro screen is held, in seconds (`--intro-secs`, default 3). The shipped
    /// original has no intro timer (KB10 §10.2); this is our approximation.
    pub intro_secs: u32,
    /// Start the 11-day story arc at this day (1–11); `0` = auto (resume the persisted day,
    /// or today). A one-off override (`--day N`), not persisted.
    pub day: u8,
    /// Story mode: play the whole 11-day arc in order (1 → 11 → 1 …) on a fixed cadence,
    /// instead of one day per real calendar day (`--story`). Starts at day 1.
    pub story: bool,
    /// In story mode, how many real seconds each story day is shown (`--story-secs`).
    pub story_secs: u32,
    /// Diagnostic mode: play only the day-specific specials in order.
    pub special_cycle: bool,
    /// Apply saved Custom display-manager overrides at startup. False preserves
    /// the settings but starts faithful original playback instead.
    pub custom_enabled: bool,
    /// Selected diagnostics population. Kept alongside `special_cycle` during migration.
    pub test_group: TestGroup,
    /// Optional day filter for diagnostic special playback. Empty means all 11 days.
    pub special_days: Vec<u8>,
    /// Force the lighting for test playback only; `Any` keeps the real clock.
    pub test_time: TestTime,
    /// Optional high/low tide override used by the scene tester and normal playback.
    pub tide_preference: TidePreference,
    /// Holiday layer forced while testing; calendar keeps the original selection.
    pub test_holiday: TestHoliday,
    /// One-based source-table scene to repeat; zero keeps group playback.
    pub test_scene: u8,
    /// One-based source-table scene where non-repeating group playback begins; zero
    /// begins at the first entry in the selected group.
    pub test_group_start: u8,
    /// One-based scene numbers personally marked for the Favorites scene set.
    pub favorite_scenes: Vec<u8>,
    /// User-editable visible catalog names, indexed by one-based scene number.
    pub scene_short_names: Vec<String>,
    /// User-editable visible catalog descriptions, indexed by one-based scene number.
    pub scene_descriptions: Vec<String>,
    /// Non-persistent diagnostics for a malformed `[SceneCatalog]` section.
    /// The reviewed built-in text is retained for any bad record.
    pub catalog_warnings: Vec<String>,
    /// Apply the following island coordinates while testing or using Custom (cutaway scenes
    /// ignore them).
    pub test_offset_enabled: bool,
    pub test_offset_x: i32,
    pub test_offset_y: i32,
    /// Scene-transition style between story runs (default: hard cut, the faithful original).
    pub transition: Transition,
}

impl Default for Config {
    fn default() -> Self {
        let (scene_short_names, scene_descriptions) = crate::settings_ui::default_scene_catalog();
        Config {
            windowed: false,
            full_size_window: false,
            always_on_top: false,
            ignore_mouse_close: false,
            return_to_primary: true,
            mute: false,
            speed: 100,
            scale: ScaleMode::Fit,
            filter: Filter::default(),
            dedither: false,
            holiday_animations: true,
            daynight: DayNight::Original,
            debug: false,
            scene_info_overlay: false,
            scene_info_corner: SceneInfoCorner::TopLeft,
            scene_info_long_description: true,
            crt_scanlines: CtrScanlines::Off,
            crt_curvature: CrtCurvature::Off,
            crt_phosphor_mask: CrtPhosphorMask::Off,
            crt_bloom: CrtBloom::Off,
            crt_color_profile: CrtColorProfile::Neutral,
            crt_horizontal_softness: CrtHorizontalSoftness::Off,
            intro: true,
            intro_secs: INTRO_SECS_DEFAULT,
            day: 0,
            story: false,
            story_secs: crate::timectl::DEFAULT_STORY_DAY_SECS,
            special_cycle: false,
            custom_enabled: true,
            test_group: TestGroup::Off,
            special_days: Vec::new(),
            test_time: TestTime::Any,
            tide_preference: TidePreference::Authentic,
            test_holiday: TestHoliday::Calendar,
            test_scene: 0,
            test_group_start: 0,
            favorite_scenes: Vec::new(),
            scene_short_names,
            scene_descriptions,
            catalog_warnings: Vec::new(),
            test_offset_enabled: false,
            test_offset_x: -90,
            test_offset_y: 5,
            transition: Transition::None,
        }
    }
}

impl Config {
    /// The user-visible scene text. Invalid/missing catalog records simply do
    /// not get displayed rather than crashing the screensaver over a typo.
    pub fn scene_catalog_text(&self, index: u8) -> Option<(&str, &str)> {
        let index = index.checked_sub(1)? as usize;
        Some((
            self.scene_short_names.get(index)?.as_str(),
            self.scene_descriptions.get(index)?.as_str(),
        ))
    }

    /// Reset only the saved Custom-profile choices. General display, window,
    /// audio, scaling, and playback preferences deliberately remain intact.
    // The native menu is Windows-only. Keep this method available for it
    // without making non-Windows CI pretend the platform-specific action is
    // dead application code.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn reset_custom_values(&mut self) {
        let defaults = Self::default();
        self.custom_enabled = false;
        self.day = defaults.day;
        self.story = defaults.story;
        self.story_secs = defaults.story_secs;
        self.special_cycle = defaults.special_cycle;
        self.test_group = defaults.test_group;
        self.special_days = defaults.special_days;
        self.test_time = defaults.test_time;
        self.daynight = defaults.daynight;
        self.tide_preference = defaults.tide_preference;
        self.test_holiday = defaults.test_holiday;
        self.test_scene = defaults.test_scene;
        self.test_group_start = defaults.test_group_start;
        self.test_offset_enabled = defaults.test_offset_enabled;
        self.test_offset_x = defaults.test_offset_x;
        self.test_offset_y = defaults.test_offset_y;
        self.transition = defaults.transition;
    }

    /// Encode the complete current configuration for a transient local Preview.
    /// This is the same documented INI text we persist to disk, so it is stable
    /// across platforms and needs no second, error-prone settings schema.
    pub fn preview_wire(&self) -> String {
        self.serialize()
    }

    /// Decode a configuration received from the loopback-only Preview channel.
    pub fn from_preview_wire(text: &str) -> Self {
        Self::parse(text)
    }

    /// Load the config from disk, falling back to defaults for anything missing.
    pub fn load() -> Config {
        let Some(canonical) = config_file() else {
            return Config::default();
        };
        Self::load_from_paths(&canonical, &legacy_config_files(&canonical))
    }

    /// Persist the config. Callers can surface the error instead of silently
    /// pretending that a setting was saved when Windows refused the write.
    pub fn save(&self) -> Result<(), String> {
        let canonical = config_file().ok_or_else(|| {
            "Castaway Continuum could not determine its settings folder.".to_owned()
        })?;
        self.save_to(&canonical)?;
        remove_legacy_config_files(&legacy_config_files(&canonical));
        Ok(())
    }

    /// Load the canonical file, or migrate one known old filename exactly once.
    /// `castaway-continuum.ini` is authoritative whenever it exists; old files can never override it.
    fn load_from_paths(canonical: &Path, legacy: &[PathBuf]) -> Config {
        if let Ok(text) = std::fs::read_to_string(canonical) {
            remove_legacy_config_files(legacy);
            let config = Config::parse(&text);
            // One-time, lossless migration: pre-catalog INIs receive the
            // editable records the next time the application opens.
            if !text.contains("scene_01_short=") {
                let _ = config.save_to(canonical);
            }
            return config;
        }
        for path in legacy {
            if let Ok(text) = std::fs::read_to_string(path) {
                let config = Config::parse(&text);
                if config.save_to(canonical).is_ok() {
                    remove_legacy_config_files(legacy);
                }
                return config;
            }
        }
        Config::default()
    }

    /// Persist to an explicit path. This is kept private so callers use the
    /// one canonical `castaway-continuum.ini` location.
    fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Could not create settings folder {}: {error}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(path, self.serialize())
            .map_err(|error| format!("Could not save {}: {error}", path.display()))
    }

    /// Parse a config file, starting from defaults and overriding recognised keys.
    fn parse(text: &str) -> Config {
        if text.trim_start().starts_with('{') {
            return Self::parse_json(text);
        }
        let mut c = Config::default();
        let has_scene_catalog = text
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case("[SceneCatalog]"));
        let mut seen_short_names = [false; SCENE_CATALOG_COUNT];
        let mut seen_descriptions = [false; SCENE_CATALOG_COUNT];
        let mut in_scene_catalog = false;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_scene_catalog = line.eq_ignore_ascii_case("[SceneCatalog]");
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            let key = key.trim();
            if in_scene_catalog {
                if let Some((scene_index, is_short_name)) = parse_scene_catalog_key(key) {
                    if is_short_name {
                        seen_short_names[scene_index] = true;
                    } else {
                        seen_descriptions[scene_index] = true;
                    }
                    if value.is_empty() {
                        c.catalog_warnings.push(format!(
                            "Scene {:02} {} is blank; using the built-in text.",
                            scene_index + 1,
                            if is_short_name {
                                "short name"
                            } else {
                                "description"
                            }
                        ));
                        continue;
                    }
                    let target = if is_short_name {
                        &mut c.scene_short_names
                    } else {
                        &mut c.scene_descriptions
                    };
                    if let Some(entry) = target.get_mut(scene_index) {
                        *entry = value.replace(['\r', '\n'], " ");
                    }
                    continue;
                }
            }
            if in_scene_catalog && key.starts_with("scene_") {
                c.catalog_warnings.push(format!(
                    "Unrecognized SceneCatalog key `{key}`; it was ignored."
                ));
                continue;
            }
            match key {
                "windowed" => {
                    if let Some(b) = parse_bool(value) {
                        c.windowed = b;
                    }
                }
                "full_size_window" => {
                    if let Some(b) = parse_bool(value) {
                        c.full_size_window = b;
                    }
                }
                "always_on_top" => {
                    if let Some(b) = parse_bool(value) {
                        c.always_on_top = b;
                    }
                }
                "ignore_mouse_close" => {
                    if let Some(b) = parse_bool(value) {
                        c.ignore_mouse_close = b;
                    }
                }
                "return_to_primary" => {
                    if let Some(b) = parse_bool(value) {
                        c.return_to_primary = b;
                    }
                }
                "mute" => {
                    if let Some(b) = parse_bool(value) {
                        c.mute = b;
                    }
                }
                "sound_enabled" => {
                    if let Some(b) = parse_bool(value) {
                        c.mute = !b;
                    }
                }
                "speed" => {
                    if let Ok(n) = value.parse::<u32>() {
                        c.speed = n.clamp(SPEED_MIN, SPEED_MAX);
                    }
                }
                "scale" => {
                    if let Some(m) = ScaleMode::parse(value) {
                        c.scale = m;
                    }
                }
                "filter" => {
                    if let Some(f) = Filter::parse(value) {
                        c.filter = f;
                    }
                }
                "dedither" => {
                    if let Some(b) = parse_bool(value) {
                        c.dedither = b;
                    }
                }
                "holiday_animations" => {
                    if let Some(b) = parse_bool(value) {
                        c.holiday_animations = b;
                    }
                }
                "debug" => {
                    if let Some(b) = parse_bool(value) {
                        c.debug = b;
                    }
                }
                "scene_info_overlay" => {
                    if let Some(b) = parse_bool(value) {
                        c.scene_info_overlay = b;
                    }
                }
                "scene_info_corner" => {
                    if let Some(corner) = SceneInfoCorner::parse(value) {
                        c.scene_info_corner = corner;
                    }
                }
                "scene_info_long_description" => {
                    if let Some(b) = parse_bool(value) {
                        c.scene_info_long_description = b;
                    }
                }
                "crt_scanlines" => {
                    if let Some(scanlines) = CtrScanlines::parse(value) {
                        c.crt_scanlines = scanlines;
                    }
                }
                "crt_curvature" => {
                    if let Some(curvature) = CrtCurvature::parse(value) {
                        c.crt_curvature = curvature;
                    }
                }
                "crt_phosphor_mask" => {
                    if let Some(mask) = CrtPhosphorMask::parse(value) {
                        c.crt_phosphor_mask = mask;
                    }
                }
                "crt_bloom" => {
                    if let Some(bloom) = CrtBloom::parse(value) {
                        c.crt_bloom = bloom;
                    }
                }
                "crt_color_profile" => {
                    if let Some(profile) = CrtColorProfile::parse(value) {
                        c.crt_color_profile = profile;
                    }
                }
                "crt_horizontal_softness" => {
                    if let Some(softness) = CrtHorizontalSoftness::parse(value) {
                        c.crt_horizontal_softness = softness;
                    }
                }
                "daynight" => {
                    if let Some(m) = DayNight::parse(value) {
                        c.daynight = m;
                    }
                }
                "intro" => {
                    if let Some(b) = parse_bool(value) {
                        c.intro = b;
                    }
                }
                "intro_secs" => {
                    if let Ok(n) = value.parse::<u32>() {
                        c.intro_secs = n.clamp(INTRO_SECS_MIN, INTRO_SECS_MAX);
                    }
                }
                "day" => {
                    if let Ok(n) = value.parse::<u8>() {
                        c.day = n.min(MAX_STORY_DAY);
                    }
                }
                "story" => {
                    if let Some(b) = parse_bool(value) {
                        c.story = b;
                    }
                }
                "story_secs" => {
                    if let Ok(n) = value.parse::<u32>() {
                        c.story_secs = n.clamp(STORY_SECS_MIN, STORY_SECS_MAX);
                    }
                }
                "special_cycle" => {
                    if let Some(b) = parse_bool(value) {
                        c.special_cycle = b;
                    }
                }
                "custom_enabled" => {
                    if let Some(enabled) = parse_bool(value) {
                        c.custom_enabled = enabled;
                    }
                }
                "test_group" => {
                    if let Some(g) = TestGroup::parse(value) {
                        c.test_group = g;
                        c.special_cycle = g == TestGroup::Special;
                    }
                }
                "special_days" => c.special_days = parse_days(value),
                "test_time" => {
                    if let Some(t) = TestTime::parse(value) {
                        c.test_time = t;
                    }
                }
                "tide_preference" | "tide" => {
                    if let Some(preference) = TidePreference::parse(value) {
                        c.tide_preference = preference;
                    }
                }
                "test_holiday" | "holiday" => {
                    if let Some(holiday) = TestHoliday::parse(value) {
                        c.test_holiday = holiday;
                    }
                }
                // INI names inside [test_playback]. The parser is deliberately
                // section-agnostic, so these are safe and keep old config keys too.
                "enabled" => {
                    if let Some(enabled) = parse_bool(value) {
                        c.special_cycle = enabled;
                        if enabled && c.test_group == TestGroup::Off {
                            c.test_group = TestGroup::Special;
                        }
                    }
                }
                "group" => {
                    if let Some(group) = TestGroup::parse(value) {
                        c.test_group = group;
                        c.special_cycle = group == TestGroup::Special;
                    }
                }
                "days" => c.special_days = parse_days(value),
                "time" => {
                    if let Some(time) = TestTime::parse(value) {
                        c.test_time = time;
                    }
                }
                "scene" => {
                    if let Ok(n) = value.parse::<u8>() {
                        c.test_scene = n.min(63);
                    }
                }
                "group_start" => {
                    if let Ok(n) = value.parse::<u8>() {
                        c.test_group_start = n.min(63);
                    }
                }
                "favorite_scenes" | "favorites" => c.favorite_scenes = parse_scenes(value),
                "offset_enabled" => {
                    if let Some(b) = parse_bool(value) {
                        c.test_offset_enabled = b;
                    }
                }
                "offset_x" => {
                    if let Ok(n) = value.parse::<i32>() {
                        c.test_offset_x = n.clamp(-640, 640);
                    }
                }
                "offset_y" => {
                    if let Ok(n) = value.parse::<i32>() {
                        c.test_offset_y = n.clamp(-480, 480);
                    }
                }
                "transition" => {
                    if let Some(t) = Transition::parse(value) {
                        c.transition = t;
                    }
                }
                _ => {}
            }
        }
        if has_scene_catalog {
            for index in 0..SCENE_CATALOG_COUNT {
                if !seen_short_names[index] {
                    c.catalog_warnings.push(format!(
                        "Scene {:02} short name is missing; using the built-in text.",
                        index + 1
                    ));
                }
                if !seen_descriptions[index] {
                    c.catalog_warnings.push(format!(
                        "Scene {:02} description is missing; using the built-in text.",
                        index + 1
                    ));
                }
            }
        }
        c
    }

    fn serialize(&self) -> String {
        let days = self
            .special_days
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let favorites = self
            .favorite_scenes
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let scene_catalog = self
            .scene_short_names
            .iter()
            .zip(&self.scene_descriptions)
            .enumerate()
            .map(|(index, (short_name, description))| {
                format!(
                    "scene_{:02}_short={}\nscene_{:02}_description={}\n",
                    index + 1,
                    short_name.replace(['\r', '\n'], " "),
                    index + 1,
                    description.replace(['\r', '\n'], " "),
                )
            })
            .collect::<String>();
        format!(
            "; Castaway Continuum configuration. Lines beginning with ; are comments.\n\
             ; Edit values after =. Examples are at the bottom.\n\
             \n[CastawayContinuum]\n\
             ; true = window, false = fullscreen screensaver.\nwindowed={}\n\
             ; true starts a windowed display maximized.\nfull_size_window={}\n\
             ; true keeps a windowed display above normal application windows.\nalways_on_top={}\n\
             ; Legacy compatibility setting (clicks now toggle display mode).\nignore_mouse_close={}\n\
             ; true moves the window to the primary display if its monitor disconnects.\nreturn_to_primary={}\n\
             ; true disables sounds.\nmute={}\n\
             ; Playback speed: 25..400; 100 is original timing.\nspeed={}\n\
             ; fit | stretch | integer | extend.\nscale={}\n\
             ; nearest | linear | xbr | xbrz.\nfilter={}\n\
             ; true smooths original dither; false preserves it.\ndedither={}\n\
             ; true enables date-based holiday props/animations.\nholiday_animations={}\n\
             \n\
             ; original = 1992 accelerated 8-hour cycle; real24h = wall-clock cycle.\ndaynight={}\n\
             ; true exposes diagnostics.\ndebug={}\n\
             ; true shows current scene information directly on the display.\nscene_info_overlay={}\n\
             ; top_left | top_right | bottom_left | bottom_right.\nscene_info_corner={}\n\
             ; true includes the longer scene description; false shows number and short name only.\nscene_info_long_description={}\n\
             ; off | light | medium | strong CRT scanlines.\ncrt_scanlines={}\n\
             ; off | gentle | curved | strong CRT screen curvature.\ncrt_curvature={}\n\
             ; off | subtle | visible RGB phosphor grille.\ncrt_phosphor_mask={}\n\
             ; off | subtle bright-pixel CRT glow.\ncrt_bloom={}\n\
             ; neutral | classic | warm CRT colour response.\ncrt_color_profile={}\n\
             ; off | light | classic horizontal CRT beam softness.\ncrt_horizontal_softness={}\n\
             ; Show original intro at startup.\nintro={}\n\
             ; Intro hold in seconds, 1..60.\nintro_secs={}\n\
             ; Override story day, 1..11; 0 resumes normal saved day.\nday={}\n\
             ; true plays 11-day arc on a synthetic schedule.\nstory={}\n\
             ; Seconds per synthetic day when story=true.\nstory_secs={}\n\
             \n; Test playback controls (testing only):\n\
             ; false preserves authentic randomized scheduling.\nenabled={}\n\
             ; true uses saved Custom overrides; false starts faithful original playback.\ncustom_enabled={}\n\
             ; off | normal (52 non-story) | special (11 story) | visitors | common | uncommon | all (63).\n\
             ; Common/uncommon are test buckets, not Sierra's original probabilities.\ngroup={}\n\
             ; Only special: selected days 1..11. Blank means all special scenes.\ndays={}\n\
             ; any uses real clock; day forces daylight; night forces night lighting.\ntime={}\n\
             ; authentic keeps Sierra's random choice; prefer_high or prefer_low apply only where low tide is allowed.\ntide_preference={}\n\
             ; calendar | none | halloween | st_patrick | christmas | new_year.\ntest_holiday={}\n\
             ; Tests are sequential.\norder=sequential\n\
             ; scene: 0 cycles the selected group; 1..63 repeats one source-table scene.\nscene={}\n\
             ; group_start: 0 starts at the first group entry; 1..63 starts at that scene.\ngroup_start={}\n\
             ; favorites: comma-separated scene numbers for the Favorites scene set.\nfavorites={}\n\
             ; Set offset_enabled=true to force X/Y island coordinates for a test scene.\n\
             ; Cutaway scenes have no island and ignore this override.\noffset_enabled={}\n\
             ; Island offset range: X -640..640, Y -480..480.\noffset_x={}\noffset_y={}\n\
             ; EXAMPLE special city/office scenes at night:\n\
             ; group=special\n; days=3,9,10\n; time=night\n\
             ; EXAMPLE all scenes in daylight:\n\
             ; group=all\n; days=\n; time=day\n\
             ; EXAMPLE authentic behavior:\n\
             ; enabled=false\n; group=off\n\
             \n\
             ; none is the faithful hard cut; dissolve enables tiled dissolve.\ntransition={}\n\
             \n[SceneCatalog]\n\
             ; Edit the short name or description after =. Keep one value per line.\n\
             ; These texts affect the Scene Selector and on-screen scene information.\n{}",
            self.windowed,
            self.full_size_window,
            self.always_on_top,
            self.ignore_mouse_close,
            self.return_to_primary,
            self.mute,
            self.speed,
            self.scale.as_str(),
            self.filter.as_str(),
            self.dedither,
            self.holiday_animations,
            self.daynight.as_str(),
            self.debug,
            self.scene_info_overlay,
            self.scene_info_corner.as_str(),
            self.scene_info_long_description,
            self.crt_scanlines.as_str(),
            self.crt_curvature.as_str(),
            self.crt_phosphor_mask.as_str(),
            self.crt_bloom.as_str(),
            self.crt_color_profile.as_str(),
            self.crt_horizontal_softness.as_str(),
            self.intro,
            self.intro_secs,
            self.day,
            self.story,
            self.story_secs,
            self.test_group != TestGroup::Off,
            self.custom_enabled,
            self.test_group.as_str(),
            days,
            self.test_time.as_str(),
            self.tide_preference.as_str(),
            self.test_holiday.as_str(),
            self.test_scene,
            self.test_group_start,
            favorites,
            self.test_offset_enabled,
            self.test_offset_x,
            self.test_offset_y,
            self.transition.as_str(),
            scene_catalog,
        )
        /*format!(
            "# Castaway Continuum — configuration.\n\
             # windowed: true runs in a window instead of fullscreen.\n\
             windowed={}\n\
             # mute: true disables sound effects.\n\
             mute={}\n\
             # speed: playback speed percent ({SPEED_MIN}–{SPEED_MAX}; 100 = original).\n\
             speed={}\n\
             # scale: window fit — fit | stretch | integer | extend (extend fills widescreen).\n\
             scale={}\n\
             # filter: pixel sampling — nearest (crisp/retro) | linear (smooth) | xbr (HD,\n\
             # dissolves dither) | xbrz (smooth edges, keeps dither).\n\
             filter={}\n\
             # dedither: true smooths the dithered sea/sky (default false = authentic look).\n\
             dedither={}\n\
             # daynight: day/night cycle — original (8h, as in 1992) | real24h (wall clock).\n\
             daynight={}\n\
             # debug: true shows diagnostics (stdout status line + on-screen HUD overlay).\n\
             debug={}\n\
             # intro: true shows the original's intro screen (INTRO.SCR) once at startup.\n\
             intro={}\n\
             # intro_secs: how long to hold the intro screen ({INTRO_SECS_MIN}–{INTRO_SECS_MAX}; default {INTRO_SECS_DEFAULT}).\n\
             intro_secs={}\n\
             # day: start the 11-day arc at this day (1–{MAX_STORY_DAY}); 0 = auto (resume/today).\n\
             day={}\n\
             # story: true plays the whole arc in order (day 1→11→1…) on a fixed cadence.\n\
             story={}\n\
             # story_secs: in story mode, real seconds per story day ({STORY_SECS_MIN}–{STORY_SECS_MAX}).\n\
             story_secs={}\n\
             # special_cycle: diagnostic mode; play only day-specific specials in order.\n\
             special_cycle={}\n\
             # transition: scene transition — none (hard cut, faithful) | dissolve (the\n\
             # original's dormant LFSR tiled dissolve, resurrected).\n\
             transition={}\n",
            self.windowed,
            self.mute,
            self.speed,
            self.scale.as_str(),
            self.filter.as_str(),
            self.dedither,
            self.daynight.as_str(),
            self.debug,
            self.intro,
            self.intro_secs,
            self.day,
            self.story,
            self.story_secs,
            self.special_cycle,
            self.transition.as_str(),
        )*/
    }

    fn parse_json(text: &str) -> Config {
        let Ok(v) = serde_json::from_str::<Value>(text) else {
            return Config::default();
        };
        let mut c = Config::default();
        c.windowed = v
            .get("windowed")
            .and_then(Value::as_bool)
            .unwrap_or(c.windowed);
        c.full_size_window = v
            .get("fullSizeWindow")
            .and_then(Value::as_bool)
            .unwrap_or(c.full_size_window);
        c.mute = v.get("mute").and_then(Value::as_bool).unwrap_or(c.mute);
        c.speed = v
            .get("speed")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .unwrap_or(c.speed)
            .clamp(SPEED_MIN, SPEED_MAX);
        c.dedither = v
            .get("dedither")
            .and_then(Value::as_bool)
            .unwrap_or(c.dedither);
        c.debug = v.get("debug").and_then(Value::as_bool).unwrap_or(c.debug);
        c.scene_info_overlay = v
            .get("sceneInfoOverlay")
            .and_then(Value::as_bool)
            .unwrap_or(c.scene_info_overlay);
        if let Some(corner) = v
            .get("sceneInfoCorner")
            .and_then(Value::as_str)
            .and_then(SceneInfoCorner::parse)
        {
            c.scene_info_corner = corner;
        }
        c.scene_info_long_description = v
            .get("sceneInfoLongDescription")
            .and_then(Value::as_bool)
            .unwrap_or(c.scene_info_long_description);
        if let Some(scanlines) = v
            .get("crtScanlines")
            .and_then(Value::as_str)
            .and_then(CtrScanlines::parse)
        {
            c.crt_scanlines = scanlines;
        }
        if let Some(curvature) = v
            .get("crtCurvature")
            .and_then(Value::as_str)
            .and_then(CrtCurvature::parse)
        {
            c.crt_curvature = curvature;
        }
        if let Some(mask) = v
            .get("crtPhosphorMask")
            .and_then(Value::as_str)
            .and_then(CrtPhosphorMask::parse)
        {
            c.crt_phosphor_mask = mask;
        }
        if let Some(bloom) = v
            .get("crtBloom")
            .and_then(Value::as_str)
            .and_then(CrtBloom::parse)
        {
            c.crt_bloom = bloom;
        }
        if let Some(profile) = v
            .get("crtColorProfile")
            .and_then(Value::as_str)
            .and_then(CrtColorProfile::parse)
        {
            c.crt_color_profile = profile;
        }
        if let Some(softness) = v
            .get("crtHorizontalSoftness")
            .and_then(Value::as_str)
            .and_then(CrtHorizontalSoftness::parse)
        {
            c.crt_horizontal_softness = softness;
        }
        c.intro = v.get("intro").and_then(Value::as_bool).unwrap_or(c.intro);
        c.intro_secs = v
            .get("introSecs")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .unwrap_or(c.intro_secs)
            .clamp(INTRO_SECS_MIN, INTRO_SECS_MAX);
        c.day = v
            .get("day")
            .and_then(Value::as_u64)
            .map(|n| n as u8)
            .unwrap_or(c.day)
            .min(MAX_STORY_DAY);
        c.story = v.get("story").and_then(Value::as_bool).unwrap_or(c.story);
        c.story_secs = v
            .get("storySecs")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .unwrap_or(c.story_secs)
            .clamp(STORY_SECS_MIN, STORY_SECS_MAX);
        if let Some(s) = v
            .get("scale")
            .and_then(Value::as_str)
            .and_then(ScaleMode::parse)
        {
            c.scale = s;
        }
        if let Some(s) = v
            .get("filter")
            .and_then(Value::as_str)
            .and_then(Filter::parse)
        {
            c.filter = s;
        }
        if let Some(s) = v
            .get("daynight")
            .and_then(Value::as_str)
            .and_then(DayNight::parse)
        {
            c.daynight = s;
        }
        if let Some(s) = v
            .get("transition")
            .and_then(Value::as_str)
            .and_then(Transition::parse)
        {
            c.transition = s;
        }
        if let Some(test) = v.get("testPlayback") {
            c.special_cycle = test
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if let Some(group) = test
                .get("group")
                .and_then(Value::as_str)
                .and_then(TestGroup::parse)
            {
                c.test_group = group;
                c.special_cycle = group == TestGroup::Special;
            } else if c.special_cycle {
                // Compatibility with the temporary pre-JSON special-cycle build.
                c.test_group = TestGroup::Special;
            }
            c.special_days = test
                .get("days")
                .and_then(Value::as_array)
                .map(|days| {
                    days.iter()
                        .filter_map(Value::as_u64)
                        .map(|n| n as u8)
                        .filter(|&n| (1..=MAX_STORY_DAY).contains(&n))
                        .collect()
                })
                .unwrap_or_default();
            c.special_days.sort_unstable();
            c.special_days.dedup();
            if let Some(time) = test
                .get("time")
                .and_then(Value::as_str)
                .and_then(TestTime::parse)
            {
                c.test_time = time;
            }
        }
        c
    }

    /// Apply CLI overrides: `--windowed`, `--mute`, `--dedither`, `--debug`, `--no-intro`,
    /// `--story`, `--speed <pct>`, `--scale <mode>`, `--filter <nearest|linear|xbr|xbrz>`,
    /// `--day <1-11>`, `--story-secs <s>`, `--special-cycle`, `--transition <none|dissolve>`. Unknown flags are
    /// ignored.
    pub fn apply_args(&mut self, args: &[String]) {
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--windowed" => self.windowed = true,
                "--always-on-top" => self.always_on_top = true,
                "--ignore-mouse-close" => self.ignore_mouse_close = true,
                "--no-return-to-primary" => self.return_to_primary = false,
                "--mute" => self.mute = true,
                "--dedither" => self.dedither = true,
                "--debug" => self.debug = true,
                "--no-intro" => self.intro = false,
                "--intro-secs" => {
                    if let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u32>().ok()) {
                        self.intro_secs = n.clamp(INTRO_SECS_MIN, INTRO_SECS_MAX);
                        i += 1;
                    }
                }
                "--story" => self.story = true,
                "--special-cycle" => {
                    self.special_cycle = true;
                    self.test_group = TestGroup::Special;
                }
                "--test-group" => {
                    if let Some(group) = args.get(i + 1).and_then(|v| TestGroup::parse(v)) {
                        self.test_group = group;
                        self.special_cycle = group == TestGroup::Special;
                        i += 1;
                    }
                }
                "--test-time" => {
                    if let Some(time) = args.get(i + 1).and_then(|v| TestTime::parse(v)) {
                        self.test_time = time;
                        i += 1;
                    }
                }
                "--special-days" => {
                    if let Some(v) = args.get(i + 1) {
                        self.special_days = parse_days(v);
                        self.special_cycle = true;
                        self.test_group = TestGroup::Special;
                        i += 1;
                    }
                }
                "--day" => {
                    if let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u8>().ok()) {
                        self.day = n.min(MAX_STORY_DAY);
                        i += 1;
                    }
                }
                "--story-secs" => {
                    if let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u32>().ok()) {
                        self.story_secs = n.clamp(STORY_SECS_MIN, STORY_SECS_MAX);
                        i += 1;
                    }
                }
                "--transition" => {
                    if let Some(t) = args.get(i + 1).and_then(|v| Transition::parse(v)) {
                        self.transition = t;
                        i += 1;
                    }
                }
                "--speed" => {
                    if let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u32>().ok()) {
                        self.speed = n.clamp(SPEED_MIN, SPEED_MAX);
                        i += 1;
                    }
                }
                "--scale" => {
                    if let Some(m) = args.get(i + 1).and_then(|v| ScaleMode::parse(v)) {
                        self.scale = m;
                        i += 1;
                    }
                }
                "--filter" => {
                    if let Some(f) = args.get(i + 1).and_then(|v| Filter::parse(v)) {
                        self.filter = f;
                        i += 1;
                    }
                }
                "--daynight" => {
                    if let Some(m) = args.get(i + 1).and_then(|v| DayNight::parse(v)) {
                        self.daynight = m;
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// How long to show a frame of `ticks` engine ticks, in milliseconds, scaled by
    /// the configured speed (1 tick = [`wilson_engine::MS_PER_TICK`] = 16 ms at 100%, the
    /// original's measured rate).
    pub fn frame_delay_ms(&self, ticks: u16) -> u64 {
        u64::from(ticks) * wilson_engine::MS_PER_TICK * 100 / u64::from(self.speed)
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Some(true),
        "false" | "no" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn parse_days(s: &str) -> Vec<u8> {
    let mut days: Vec<u8> = s
        .split(',')
        .filter_map(|v| v.trim().parse::<u8>().ok())
        .filter(|n| (1..=MAX_STORY_DAY).contains(n))
        .collect();
    days.sort_unstable();
    days.dedup();
    days
}

fn parse_scenes(s: &str) -> Vec<u8> {
    let mut scenes: Vec<u8> = s
        .split(',')
        .filter_map(|value| value.trim().parse::<u8>().ok())
        .filter(|&scene| (1..=63).contains(&scene))
        .collect();
    scenes.sort_unstable();
    scenes.dedup();
    scenes
}

/// Decode `scene_01_short` and `scene_01_description` keys into zero-based
/// catalog positions. The intentionally strict spelling prevents random user
/// parameters from becoming accidental catalog entries.
fn parse_scene_catalog_key(key: &str) -> Option<(usize, bool)> {
    let suffix = key.strip_prefix("scene_")?;
    let (number, field) = suffix.split_once('_')?;
    let index = number.parse::<usize>().ok()?.checked_sub(1)?;
    (index < SCENE_CATALOG_COUNT)
        .then_some((index, field == "short"))
        .filter(|_| matches!(field, "short" | "description"))
}

/// The portable configuration always lives beside the running EXE/SCR. This keeps
/// a packaged screensaver self-contained and makes its settings easy to find.
pub fn config_file() -> Option<PathBuf> {
    config_file_for_executable(&std::env::current_exe().ok()?)
}

fn config_file_for_executable(executable: &Path) -> Option<PathBuf> {
    Some(executable.parent()?.join("castaway-continuum.ini"))
}

/// Files written by earlier builds. The old per-user `wilson.ini` is retained as
/// a safety backup; obsolete local `config.json`/`config.txt` files are retired.
fn legacy_config_files(canonical: &Path) -> Vec<PathBuf> {
    let dir = canonical.parent().unwrap_or_else(|| Path::new("."));
    let mut paths = vec![
        dir.join("config.json"),
        dir.join("config.txt"),
        dir.join("wilson.ini"),
    ];
    // Pre-portable builds stored their canonical INI under `%APPDATA%` on
    // Windows (or the XDG state directory elsewhere). Migrate it exactly once
    // when a new EXE/SCR-local INI has not been created yet.
    if let Some(old_config) = crate::state::state_dir().map(|dir| dir.join("wilson.ini")) {
        if old_config != canonical {
            paths.push(old_config);
        }
    }
    paths
}

fn remove_legacy_config_files(paths: &[PathBuf]) {
    for path in paths {
        // Keep the old per-user INI intact after copying it beside the EXE/SCR.
        // A user may have more than one portable installation, and deleting the
        // only prior settings copy is a rotten migration strategy.
        if path
            .file_name()
            .is_some_and(|name| name == "castaway-continuum.ini")
        {
            continue;
        }
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_screensaver_friendly() {
        let c = Config::default();
        assert!(!c.windowed); // fullscreen
        assert!(!c.mute);
        assert_eq!(c.speed, 100);
        assert_eq!(c.scale, ScaleMode::Fit);
        assert_eq!(c.filter, Filter::Linear); // linear by default
        assert!(!c.dedither); // keep the authentic dither by default
        assert!(!c.debug); // no diagnostics by default
        assert!(c.intro); // the intro screen is shown by default (like the original)
        assert_eq!(c.day, 0); // auto: resume the persisted/real day
        assert!(!c.story); // real calendar by default, not story mode
        assert_eq!(c.story_secs, crate::timectl::DEFAULT_STORY_DAY_SECS);
        assert!(!c.special_cycle);
        assert_eq!(c.test_time, TestTime::Any);
        assert_eq!(c.transition, Transition::None); // hard cut by default (faithful)
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn reset_custom_values_keeps_display_preferences() {
        let mut c = Config::default();
        // These are the Display-menu preferences the user deliberately keeps.
        c.scale = ScaleMode::Integer;
        c.filter = Filter::Xbrz;
        c.dedither = true;
        c.always_on_top = true;
        c.full_size_window = true;
        c.return_to_primary = false;

        // And these are Custom values which must be reset.
        c.custom_enabled = true;
        c.test_time = TestTime::Night;
        c.tide_preference = TidePreference::PreferLow;
        c.test_scene = 17;
        c.favorite_scenes = vec![7, 17, 63];
        c.test_offset_enabled = true;
        c.test_offset_x = 125;
        c.test_offset_y = -45;

        c.reset_custom_values();

        assert_eq!(c.scale, ScaleMode::Integer);
        assert_eq!(c.filter, Filter::Xbrz);
        assert!(c.dedither);
        assert!(c.always_on_top);
        assert!(c.full_size_window);
        assert!(!c.return_to_primary);
        assert!(!c.custom_enabled);
        assert_eq!(c.test_time, TestTime::Any);
        assert_eq!(c.tide_preference, TidePreference::Authentic);
        assert_eq!(c.test_scene, 0);
        assert_eq!(c.favorite_scenes, vec![7, 17, 63]);
        assert!(!c.test_offset_enabled);
    }

    #[test]
    fn parse_round_trips() {
        let (scene_short_names, scene_descriptions) = crate::settings_ui::default_scene_catalog();
        let c = Config {
            windowed: true,
            full_size_window: true,
            always_on_top: true,
            ignore_mouse_close: true,
            return_to_primary: true,
            mute: true,
            speed: 250,
            scale: ScaleMode::Integer,
            filter: Filter::Nearest,
            dedither: true,
            holiday_animations: false,
            daynight: DayNight::Real24h,
            debug: true,
            scene_info_overlay: true,
            scene_info_corner: SceneInfoCorner::TopLeft,
            scene_info_long_description: true,
            crt_scanlines: CtrScanlines::Medium,
            crt_curvature: CrtCurvature::Curved,
            crt_phosphor_mask: CrtPhosphorMask::Subtle,
            crt_bloom: CrtBloom::Subtle,
            crt_color_profile: CrtColorProfile::Classic,
            crt_horizontal_softness: CrtHorizontalSoftness::Light,
            intro: false,
            intro_secs: 5,
            day: 7,
            story: true,
            story_secs: 120,
            special_cycle: true,
            custom_enabled: true,
            test_group: TestGroup::Special,
            special_days: vec![3, 9, 10],
            test_time: TestTime::Night,
            tide_preference: TidePreference::PreferLow,
            test_holiday: TestHoliday::Calendar,
            test_scene: 17,
            test_group_start: 18,
            favorite_scenes: vec![7, 17, 63],
            scene_short_names,
            scene_descriptions,
            catalog_warnings: Vec::new(),
            test_offset_enabled: true,
            test_offset_x: -120,
            test_offset_y: -35,
            transition: Transition::Dissolve,
        };
        assert_eq!(Config::parse(&c.serialize()), c);
    }

    #[test]
    fn scene_catalog_is_editable_and_invalid_records_keep_defaults() {
        let c = Config::parse(
            "[SceneCatalog]\nscene_01_short=Custom dive\nscene_01_description=My edited description.\nscene_99_short=Ignored\n",
        );
        assert_eq!(
            c.scene_catalog_text(1),
            Some(("Custom dive", "My edited description."))
        );
        assert_eq!(c.scene_catalog_text(2), Some(("Gull Nest on Head", "A gull lands on Johnny's head and starts a nest. Johnny misses it with a club and hits himself.")));
        assert!(!c.catalog_warnings.is_empty());
    }

    #[test]
    fn existing_ini_is_upgraded_with_the_editable_scene_catalog() {
        let root =
            std::env::temp_dir().join(format!("wilson-catalog-migration-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let canonical = root.join("castaway-continuum.ini");
        std::fs::write(&canonical, "enabled=true\ntide=low\n").unwrap();

        let config = Config::load_from_paths(&canonical, &[]);
        assert_eq!(config.tide_preference, TidePreference::PreferLow);
        let migrated = std::fs::read_to_string(&canonical).unwrap();
        assert!(migrated.contains("[SceneCatalog]"));
        assert!(migrated.contains("scene_01_short=Dive Scorecards"));
        assert!(migrated.contains("scene_63_description="));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn portable_ini_sits_beside_the_executable_or_screensaver() {
        let exe = Path::new(r"C:\Games\Johnny Castaway\JohnnyCastaway.scr");
        assert_eq!(
            config_file_for_executable(exe),
            Some(PathBuf::from(
                r"C:\Games\Johnny Castaway\castaway-continuum.ini"
            ))
        );
    }

    #[test]
    fn legacy_json_migrates_to_one_canonical_ini() {
        let root =
            std::env::temp_dir().join(format!("wilson_config_migration_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let canonical = root.join("castaway-continuum.ini");
        let json = root.join("config.json");
        let text = root.join("config.txt");
        std::fs::write(&json, r#"{"mute":true,"speed":200}"#).unwrap();
        std::fs::write(&text, "mute=false\nspeed=50\n").unwrap();

        let config = Config::load_from_paths(&canonical, &[json.clone(), text.clone()]);
        assert!(config.mute);
        assert_eq!(config.speed, 200);
        assert!(canonical.exists());
        assert!(!json.exists());
        assert!(!text.exists());
        assert_eq!(
            Config::parse(&std::fs::read_to_string(&canonical).unwrap()),
            config
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn canonical_ini_wins_over_and_retires_legacy_files() {
        let root =
            std::env::temp_dir().join(format!("wilson_config_canonical_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let canonical = root.join("castaway-continuum.ini");
        let json = root.join("config.json");
        std::fs::write(&canonical, "mute=true\nspeed=175\n").unwrap();
        std::fs::write(&json, r#"{"mute":false,"speed":25}"#).unwrap();

        let config = Config::load_from_paths(&canonical, &[json.clone(), root.join("config.txt")]);
        assert!(config.mute);
        assert_eq!(config.speed, 175);
        assert!(!json.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parse_is_lenient() {
        let c = Config::parse(
            "# comment\n\nwindowed=yes\nmute=off\nbogus=1\nscale=stretch\nfilter=crisp\ndaynight=24h\n",
        );
        assert!(c.windowed);
        assert!(!c.mute);
        assert_eq!(c.scale, ScaleMode::Stretch);
        assert_eq!(c.filter, Filter::Nearest); // "crisp" synonym
        assert_eq!(c.daynight, DayNight::Real24h);
        assert_eq!(c.speed, 100); // untouched → default
    }

    #[test]
    fn json_test_days_are_sorted_and_deduplicated() {
        let c = Config::parse(
            r#"{"testPlayback":{"enabled":true,"group":"special","days":[10,3,3,99]}}"#,
        );
        assert!(c.special_cycle);
        assert_eq!(c.test_group, TestGroup::Special);
        assert_eq!(c.special_days, vec![3, 10]);
    }

    #[test]
    fn speed_is_clamped() {
        assert_eq!(Config::parse("speed=5\n").speed, SPEED_MIN);
        assert_eq!(Config::parse("speed=9999\n").speed, SPEED_MAX);
    }

    #[test]
    fn story_options_are_clamped() {
        assert_eq!(Config::parse("day=99\n").day, MAX_STORY_DAY); // clamp to the 11-day arc
        assert_eq!(Config::parse("story_secs=1\n").story_secs, STORY_SECS_MIN);
        assert_eq!(
            Config::parse("story_secs=999999\n").story_secs,
            STORY_SECS_MAX
        );
    }

    #[test]
    fn cli_overrides_win() {
        let mut c = Config::default();
        let args: Vec<String> = [
            "x",
            "--windowed",
            "--mute",
            "--speed",
            "200",
            "--scale",
            "integer",
            "--filter",
            "nearest",
            "--dedither",
            "--debug",
            "--daynight",
            "real24h",
            "--no-intro",
            "--story",
            "--day",
            "5",
            "--story-secs",
            "120",
            "--special-cycle",
            "--transition",
            "dissolve",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        c.apply_args(&args);
        assert!(c.windowed);
        assert!(c.mute);
        assert_eq!(c.speed, 200);
        assert_eq!(c.scale, ScaleMode::Integer);
        assert_eq!(c.filter, Filter::Nearest);
        assert!(c.dedither);
        assert!(c.debug);
        assert_eq!(c.daynight, DayNight::Real24h);
        assert!(!c.intro); // --no-intro
        assert!(c.story); // --story
        assert_eq!(c.day, 5); // --day 5
        assert_eq!(c.story_secs, 120); // --story-secs 120
        assert!(c.special_cycle);
        assert_eq!(c.transition, Transition::Dissolve); // --transition dissolve
    }

    #[test]
    fn cli_speed_is_clamped_and_bad_values_ignored() {
        let mut c = Config::default();
        c.apply_args(&["x".into(), "--speed".into(), "1000".into()]);
        assert_eq!(c.speed, SPEED_MAX);
        let mut c2 = Config::default();
        c2.apply_args(&["x".into(), "--speed".into(), "abc".into()]);
        assert_eq!(c2.speed, 100); // unchanged
    }

    #[test]
    fn frame_delay_scales_with_speed() {
        let mut c = Config::default();
        assert_eq!(c.frame_delay_ms(6), 96); // 6 ticks * 16 ms at 100% (the original's rate)
        c.speed = 200;
        assert_eq!(c.frame_delay_ms(6), 48); // twice as fast
        c.speed = 50;
        assert_eq!(c.frame_delay_ms(6), 192); // half speed
    }
}
