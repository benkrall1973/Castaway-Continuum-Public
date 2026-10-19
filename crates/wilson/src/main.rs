// SPDX-License-Identifier: GPL-3.0-or-later
//! Castaway Continuum — the Johnny Castaway screensaver, as a live window.
//!
//! Runs the [`wilson_engine`] runtime on the **original** game data and presents each
//! composited frame with `softbuffer` in a `winit` window. Any real key press quits;
//! a mouse click switches between framed and fullscreen display. Lone modifier keys are
//! ignored - see [`key_dismisses`]. Runs fullscreen by default (use `--windowed` for dev).
//!
//! It needs the original Johnny Castaway data (`RESOURCE.MAP` + `RESOURCE.001`):
//! - `wilson --data <dir>` — load the data from `<dir>`.
//! - `wilson` — auto-detects the data in the working directory or next to the executable.
//! - `wilson --windowed --mute --speed <pct> --scale fit|stretch|integer|extend
//!   --filter nearest|linear|xbr|xbrz --dedither --no-intro --intro-secs <s> --day <1-11>
//!   --story --story-secs <s> --test-group off|normal|special|visitors|common|uncommon|all --special-days <1,3,5>
//!   --transition none|dissolve` - options (`extend` fills widescreen;
//!   `--story` plays the arc in order; `--transition dissolve` = the original's tiled dissolve).
//! - Windows screensaver verbs: `/s` (show), `/c` (opens the sibling EXE for configuration),
//!   `/p <hwnd>` (preview embedded in the configuration pane — Windows only).

// A screensaver is a GUI program: on Windows, link it for the "windows" subsystem so the
// `.scr` doesn't pop a black console window when launched from the Screen Saver settings.
// Kept in debug builds so `--debug` logging still has a console to print to.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(feature = "embed-data"))]
mod assets;
mod audio;
mod config;
#[cfg(feature = "embed-data")]
mod embedded;
mod font;
mod pace;
mod scale;
mod settings_ui;
mod state;
mod stats;
mod timectl;

use std::net::UdpSocket;
use std::num::NonZeroU32;
#[cfg(windows)]
use std::process::Child;
use std::rc::Rc;
#[cfg(windows)]
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wilson_dgds::Archive;
use wilson_engine::{
    clock, DayNight, Director, Holiday, Show, TestSceneGroup, TidePreference, STORY_SCENES,
    WAVE_PERIOD_TICKS,
};
use winit::event::{ElementState, Event, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::window::{Fullscreen, WindowBuilder, WindowLevel};

/// Whether a keyboard event should dismiss the screensaver (like a real one: any real key
/// press quits). Ignores key *releases*, the **synthetic** key events the OS delivers when
/// a window gains/loses focus, and lone **modifier** keys (Alt/AltGr/Ctrl/Shift/Super/Meta/
/// locks/Fn). This matters on Windows: a borderless-fullscreen window receives a spurious
/// `AltGraph` press the instant it grabs focus, which would otherwise close it immediately.
fn key_dismisses(logical_key: &Key, state: ElementState, is_synthetic: bool) -> bool {
    if state != ElementState::Pressed || is_synthetic {
        return false;
    }
    !matches!(
        logical_key,
        Key::Named(
            NamedKey::Alt
                | NamedKey::AltGraph
                | NamedKey::Control
                | NamedKey::Shift
                | NamedKey::Super
                | NamedKey::Meta
                | NamedKey::Hyper
                | NamedKey::Symbol
                | NamedKey::Fn
                | NamedKey::FnLock
                | NamedKey::CapsLock
                | NamedKey::NumLock
                | NamedKey::ScrollLock
        )
    )
}

/// Seed a fresh original run. In particular, leaving Custom mode must not
/// recreate the exact same deterministic placement that was active before the
/// user pinned the island.
fn fresh_run_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

/// Settings writes must not crash a screensaver, but they must not fail
/// silently either. The selector can show the same error inline; the main
/// display has no status surface, so retain it in the debug/error stream.
fn save_config_or_report(cfg: &config::Config) {
    if let Err(error) = cfg.save() {
        eprintln!("[castaway-continuum] {error}");
    }
}

fn report_persistence_error(result: Result<(), String>) {
    if let Err(error) = result {
        eprintln!("[castaway-continuum] {error}");
    }
}

// Native Windows menus appear whenever the display is in its framed window mode.
#[cfg(windows)]
static NATIVE_MENU_COMMAND: AtomicU16 = AtomicU16::new(0);

/// Handle of the Display popup in the currently installed native menu.
#[cfg(windows)]
static DISPLAY_MENU_HANDLE: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
const MENU_EXIT: usize = 0x7A01;
#[cfg(windows)]
const MENU_TOGGLE_ALWAYS_ON_TOP: usize = 0x7A02;
#[cfg(windows)]
const MENU_TOGGLE_IGNORE_MOUSE_CLOSE: usize = 0x7A03;
#[cfg(windows)]
const MENU_TOGGLE_RETURN_TO_PRIMARY: usize = 0x7A04;
#[cfg(windows)]
const MENU_TOGGLE_DEDITHER: usize = 0x7A05;
#[cfg(windows)]
const MENU_TOGGLE_SCENE_INFO: usize = 0x7A06;
#[cfg(windows)]
const MENU_SCENE_INFO_TOP_LEFT: usize = 0x7A14;
#[cfg(windows)]
const MENU_SCENE_INFO_TOP_RIGHT: usize = 0x7A15;
#[cfg(windows)]
const MENU_SCENE_INFO_BOTTOM_LEFT: usize = 0x7A16;
#[cfg(windows)]
const MENU_SCENE_INFO_BOTTOM_RIGHT: usize = 0x7A17;
#[cfg(windows)]
const MENU_TOGGLE_SCENE_INFO_LONG_DESCRIPTION: usize = 0x7A18;
#[cfg(windows)]
const MENU_CRT_SCANLINES_OFF: usize = 0x7A19;
#[cfg(windows)]
const MENU_CRT_SCANLINES_LIGHT: usize = 0x7A1A;
#[cfg(windows)]
const MENU_CRT_SCANLINES_MEDIUM: usize = 0x7A1B;
#[cfg(windows)]
const MENU_CRT_SCANLINES_STRONG: usize = 0x7A1C;
#[cfg(windows)]
const MENU_CRT_CURVATURE_OFF: usize = 0x7A24;
#[cfg(windows)]
const MENU_CRT_CURVATURE_GENTLE: usize = 0x7A25;
#[cfg(windows)]
const MENU_CRT_CURVATURE_CURVED: usize = 0x7A26;
#[cfg(windows)]
const MENU_CRT_CURVATURE_STRONG: usize = 0x7A27;
#[cfg(windows)]
const MENU_CRT_PHOSPHOR_MASK_OFF: usize = 0x7A28;
#[cfg(windows)]
const MENU_CRT_PHOSPHOR_MASK_SUBTLE: usize = 0x7A29;
#[cfg(windows)]
const MENU_CRT_PHOSPHOR_MASK_VISIBLE: usize = 0x7A2A;
#[cfg(windows)]
const MENU_CRT_BLOOM_OFF: usize = 0x7A2B;
#[cfg(windows)]
const MENU_CRT_BLOOM_SUBTLE: usize = 0x7A2C;
#[cfg(windows)]
const MENU_CRT_COLOR_NEUTRAL: usize = 0x7A2D;
#[cfg(windows)]
const MENU_CRT_COLOR_CLASSIC: usize = 0x7A2E;
#[cfg(windows)]
const MENU_CRT_COLOR_WARM: usize = 0x7A2F;
#[cfg(windows)]
const MENU_CRT_SOFTNESS_OFF: usize = 0x7A53;
#[cfg(windows)]
const MENU_CRT_SOFTNESS_LIGHT: usize = 0x7A54;
#[cfg(windows)]
const MENU_CRT_SOFTNESS_CLASSIC: usize = 0x7A55;
#[cfg(windows)]
const MENU_CUSTOM: usize = 0x7A07;
#[cfg(windows)]
const MENU_TOGGLE_FULL_SIZE_WINDOW: usize = 0x7A08;
#[cfg(windows)]
const MENU_PROFILE_CUSTOM: usize = 0x7A09;
#[cfg(windows)]
const MENU_PROFILE_ORIGINAL: usize = 0x7A0A;
#[cfg(windows)]
const MENU_ISLAND_LOCATION: usize = 0x7A0B;
#[cfg(windows)]
const MENU_TOGGLE_ISLAND_LOCATION: usize = 0x7A0C;
#[cfg(windows)]
const MENU_RESET_CUSTOM: usize = 0x7A0D;
#[cfg(windows)]
const MENU_CLEAR_FAVORITES: usize = 0x7A0E;
#[cfg(windows)]
const MENU_SCALE_FIT: usize = 0x7A10;
#[cfg(windows)]
const MENU_SCALE_STRETCH: usize = 0x7A11;
#[cfg(windows)]
const MENU_SCALE_INTEGER: usize = 0x7A12;
#[cfg(windows)]
const MENU_SCALE_EXTEND: usize = 0x7A13;
#[cfg(windows)]
const MENU_FILTER_NEAREST: usize = 0x7A20;
#[cfg(windows)]
const MENU_FILTER_LINEAR: usize = 0x7A21;
#[cfg(windows)]
const MENU_FILTER_XBR: usize = 0x7A22;
#[cfg(windows)]
const MENU_FILTER_XBRZ: usize = 0x7A23;
#[cfg(windows)]
const MENU_LIGHTING_AUTHENTIC: usize = 0x7A30;
#[cfg(windows)]
const MENU_LIGHTING_DAY: usize = 0x7A31;
#[cfg(windows)]
const MENU_LIGHTING_NIGHT: usize = 0x7A32;
#[cfg(windows)]
const MENU_LIGHTING_PC_TIME: usize = 0x7A3C;
#[cfg(windows)]
const MENU_TIDE_AUTHENTIC: usize = 0x7A33;
#[cfg(windows)]
const MENU_TIDE_HIGH: usize = 0x7A34;
#[cfg(windows)]
const MENU_TIDE_LOW: usize = 0x7A35;
#[cfg(windows)]
const MENU_HOLIDAY_CALENDAR: usize = 0x7A36;
#[cfg(windows)]
const MENU_HOLIDAY_OFF: usize = 0x7A37;
#[cfg(windows)]
const MENU_HOLIDAY_HALLOWEEN: usize = 0x7A38;
#[cfg(windows)]
const MENU_HOLIDAY_ST_PATRICK: usize = 0x7A39;
#[cfg(windows)]
const MENU_HOLIDAY_CHRISTMAS: usize = 0x7A3A;
#[cfg(windows)]
const MENU_HOLIDAY_NEW_YEAR: usize = 0x7A3B;
#[cfg(windows)]
const MENU_TOGGLE_MUTE: usize = 0x7A40;
#[cfg(windows)]
const MENU_SPEED_25: usize = 0x7A41;
#[cfg(windows)]
const MENU_SPEED_50: usize = 0x7A42;
#[cfg(windows)]
const MENU_SPEED_100: usize = 0x7A43;
#[cfg(windows)]
const MENU_SPEED_150: usize = 0x7A44;
#[cfg(windows)]
const MENU_SPEED_200: usize = 0x7A45;
#[cfg(windows)]
const MENU_SPEED_300: usize = 0x7A46;
#[cfg(windows)]
const MENU_SPEED_400: usize = 0x7A47;
#[cfg(windows)]
const MENU_TOGGLE_INTRO: usize = 0x7A50;
#[cfg(windows)]
const MENU_TRANSITION_CUT: usize = 0x7A51;
#[cfg(windows)]
const MENU_TRANSITION_DISSOLVE: usize = 0x7A52;
#[cfg(windows)]
const MENU_ABOUT: usize = 0x7A60;

/// Commands which edit the optional Custom playback profile. They are kept
/// unavailable while Default mode is selected, rather than letting a menu
/// click appear to work while the runtime deliberately ignores its setting.
#[cfg(windows)]
fn is_custom_menu_command(command: usize) -> bool {
    matches!(
        command,
        MENU_RESET_CUSTOM
            | MENU_CLEAR_FAVORITES
            | MENU_CUSTOM
            | MENU_ISLAND_LOCATION
            | MENU_TOGGLE_ISLAND_LOCATION
            | MENU_LIGHTING_AUTHENTIC
            | MENU_LIGHTING_PC_TIME
            | MENU_LIGHTING_DAY
            | MENU_LIGHTING_NIGHT
            | MENU_TIDE_AUTHENTIC
            | MENU_TIDE_HIGH
            | MENU_TIDE_LOW
            | MENU_HOLIDAY_CALENDAR
            | MENU_HOLIDAY_OFF
            | MENU_HOLIDAY_HALLOWEEN
            | MENU_HOLIDAY_ST_PATRICK
            | MENU_HOLIDAY_CHRISTMAS
            | MENU_HOLIDAY_NEW_YEAR
    )
}

#[cfg(windows)]
#[repr(C)]
struct NativePoint {
    x: i32,
    y: i32,
}

#[cfg(windows)]
#[repr(C)]
struct NativeMessage {
    hwnd: isize,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    point: NativePoint,
    l_private: u32,
}

#[cfg(windows)]
fn install_native_menu(window: &winit::window::Window, cfg: &config::Config) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    unsafe extern "system" {
        fn CreateMenu() -> isize;
        fn CreatePopupMenu() -> isize;
        fn AppendMenuW(menu: isize, flags: u32, item: usize, text: *const u16) -> i32;
        fn DrawMenuBar(hwnd: isize) -> i32;
        fn SetMenu(hwnd: isize, menu: isize) -> i32;
    }
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = handle.hwnd.get();
    // SAFETY: Windows owns the menu tree after SetMenu succeeds. AppendMenuW
    // copies the supplied strings during each call.
    unsafe {
        let bar = CreateMenu();
        if bar == 0 {
            return;
        }
        let text = |value: &str| value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let item = |menu, label: &str, command, checked| {
            let label = text(label);
            let flags = if checked { 0x0008 } else { 0 }; // MF_CHECKED
            let _ = AppendMenuW(menu, flags, command, label.as_ptr());
        };
        let separator = |menu| {
            let _ = AppendMenuW(menu, 0x0800, 0, std::ptr::null()); // MF_SEPARATOR
        };
        let submenu = |menu, label: &str, child, enabled| {
            let label = text(label);
            let flags = 0x0010 | if enabled { 0 } else { 0x0001 }; // MF_POPUP | MF_GRAYED
            let _ = AppendMenuW(menu, flags, child as usize, label.as_ptr());
        };

        let file = CreatePopupMenu();
        item(file, "Exit", MENU_EXIT, false);
        submenu(bar, "File", file, true);

        let display = CreatePopupMenu();
        item(
            display,
            "Classic Mode",
            MENU_PROFILE_ORIGINAL,
            !cfg.custom_enabled,
        );
        item(
            display,
            "Custom Mode",
            MENU_PROFILE_CUSTOM,
            cfg.custom_enabled,
        );
        if cfg.custom_enabled {
            item(display, "Reset custom values...", MENU_RESET_CUSTOM, false);
            item(display, "Clear favorites...", MENU_CLEAR_FAVORITES, false);
            separator(display);
            item(display, "Custom Settings...", MENU_CUSTOM, false);
            separator(display);
        }
        let scale = CreatePopupMenu();
        item(
            scale,
            "Fit",
            MENU_SCALE_FIT,
            cfg.scale == scale::ScaleMode::Fit,
        );
        item(
            scale,
            "Stretch",
            MENU_SCALE_STRETCH,
            cfg.scale == scale::ScaleMode::Stretch,
        );
        item(
            scale,
            "Integer",
            MENU_SCALE_INTEGER,
            cfg.scale == scale::ScaleMode::Integer,
        );
        item(
            scale,
            "Extend",
            MENU_SCALE_EXTEND,
            cfg.scale == scale::ScaleMode::Extend,
        );
        submenu(display, "Scale", scale, true);
        let filter = CreatePopupMenu();
        item(
            filter,
            "Nearest",
            MENU_FILTER_NEAREST,
            cfg.filter == scale::Filter::Nearest,
        );
        item(
            filter,
            "Linear",
            MENU_FILTER_LINEAR,
            cfg.filter == scale::Filter::Linear,
        );
        item(
            filter,
            "xBR",
            MENU_FILTER_XBR,
            cfg.filter == scale::Filter::Xbr,
        );
        item(
            filter,
            "xBRZ",
            MENU_FILTER_XBRZ,
            cfg.filter == scale::Filter::Xbrz,
        );
        submenu(display, "Pixel filter", filter, true);
        item(
            display,
            "Smooth dithered sea and sky",
            MENU_TOGGLE_DEDITHER,
            cfg.dedither,
        );
        let scanlines = CreatePopupMenu();
        item(
            scanlines,
            "Off",
            MENU_CRT_SCANLINES_OFF,
            cfg.crt_scanlines == config::CtrScanlines::Off,
        );
        item(
            scanlines,
            "Light",
            MENU_CRT_SCANLINES_LIGHT,
            cfg.crt_scanlines == config::CtrScanlines::Light,
        );
        item(
            scanlines,
            "Medium",
            MENU_CRT_SCANLINES_MEDIUM,
            cfg.crt_scanlines == config::CtrScanlines::Medium,
        );
        item(
            scanlines,
            "Strong",
            MENU_CRT_SCANLINES_STRONG,
            cfg.crt_scanlines == config::CtrScanlines::Strong,
        );
        submenu(display, "CRT scanlines", scanlines, true);
        let curvature = CreatePopupMenu();
        item(
            curvature,
            "Off",
            MENU_CRT_CURVATURE_OFF,
            cfg.crt_curvature == config::CrtCurvature::Off,
        );
        item(
            curvature,
            "Gentle",
            MENU_CRT_CURVATURE_GENTLE,
            cfg.crt_curvature == config::CrtCurvature::Gentle,
        );
        item(
            curvature,
            "Curved",
            MENU_CRT_CURVATURE_CURVED,
            cfg.crt_curvature == config::CrtCurvature::Curved,
        );
        item(
            curvature,
            "Strong",
            MENU_CRT_CURVATURE_STRONG,
            cfg.crt_curvature == config::CrtCurvature::Strong,
        );
        submenu(display, "CRT curve", curvature, true);
        let phosphor_mask = CreatePopupMenu();
        item(
            phosphor_mask,
            "Off",
            MENU_CRT_PHOSPHOR_MASK_OFF,
            cfg.crt_phosphor_mask == config::CrtPhosphorMask::Off,
        );
        item(
            phosphor_mask,
            "Subtle",
            MENU_CRT_PHOSPHOR_MASK_SUBTLE,
            cfg.crt_phosphor_mask == config::CrtPhosphorMask::Subtle,
        );
        item(
            phosphor_mask,
            "Visible",
            MENU_CRT_PHOSPHOR_MASK_VISIBLE,
            cfg.crt_phosphor_mask == config::CrtPhosphorMask::Visible,
        );
        submenu(display, "CRT phosphor mask", phosphor_mask, true);
        let bloom = CreatePopupMenu();
        item(
            bloom,
            "Off",
            MENU_CRT_BLOOM_OFF,
            cfg.crt_bloom == config::CrtBloom::Off,
        );
        item(
            bloom,
            "Subtle",
            MENU_CRT_BLOOM_SUBTLE,
            cfg.crt_bloom == config::CrtBloom::Subtle,
        );
        submenu(display, "CRT bloom", bloom, true);
        let color_profile = CreatePopupMenu();
        item(
            color_profile,
            "Neutral",
            MENU_CRT_COLOR_NEUTRAL,
            cfg.crt_color_profile == config::CrtColorProfile::Neutral,
        );
        item(
            color_profile,
            "Classic CRT",
            MENU_CRT_COLOR_CLASSIC,
            cfg.crt_color_profile == config::CrtColorProfile::Classic,
        );
        item(
            color_profile,
            "Warm TV",
            MENU_CRT_COLOR_WARM,
            cfg.crt_color_profile == config::CrtColorProfile::Warm,
        );
        submenu(display, "CRT color profile", color_profile, true);
        let horizontal_softness = CreatePopupMenu();
        item(
            horizontal_softness,
            "Off",
            MENU_CRT_SOFTNESS_OFF,
            cfg.crt_horizontal_softness == config::CrtHorizontalSoftness::Off,
        );
        item(
            horizontal_softness,
            "Light",
            MENU_CRT_SOFTNESS_LIGHT,
            cfg.crt_horizontal_softness == config::CrtHorizontalSoftness::Light,
        );
        item(
            horizontal_softness,
            "Classic",
            MENU_CRT_SOFTNESS_CLASSIC,
            cfg.crt_horizontal_softness == config::CrtHorizontalSoftness::Classic,
        );
        submenu(
            display,
            "CRT horizontal softness",
            horizontal_softness,
            true,
        );
        item(
            display,
            "Show scene information",
            MENU_TOGGLE_SCENE_INFO,
            cfg.scene_info_overlay,
        );
        item(
            display,
            "Show long scene description",
            MENU_TOGGLE_SCENE_INFO_LONG_DESCRIPTION,
            cfg.scene_info_long_description,
        );
        let scene_info_position = CreatePopupMenu();
        item(
            scene_info_position,
            "Top left",
            MENU_SCENE_INFO_TOP_LEFT,
            cfg.scene_info_corner == config::SceneInfoCorner::TopLeft,
        );
        item(
            scene_info_position,
            "Top right",
            MENU_SCENE_INFO_TOP_RIGHT,
            cfg.scene_info_corner == config::SceneInfoCorner::TopRight,
        );
        item(
            scene_info_position,
            "Bottom left",
            MENU_SCENE_INFO_BOTTOM_LEFT,
            cfg.scene_info_corner == config::SceneInfoCorner::BottomLeft,
        );
        item(
            scene_info_position,
            "Bottom right",
            MENU_SCENE_INFO_BOTTOM_RIGHT,
            cfg.scene_info_corner == config::SceneInfoCorner::BottomRight,
        );
        submenu(
            display,
            "Scene information position",
            scene_info_position,
            true,
        );
        separator(display);
        item(
            display,
            "Always on top",
            MENU_TOGGLE_ALWAYS_ON_TOP,
            cfg.always_on_top,
        );
        item(
            display,
            "Full-size window by default",
            MENU_TOGGLE_FULL_SIZE_WINDOW,
            cfg.full_size_window,
        );
        item(
            display,
            "Return to primary display",
            MENU_TOGGLE_RETURN_TO_PRIMARY,
            cfg.return_to_primary,
        );
        submenu(bar, "Display", display, true);

        let playback = CreatePopupMenu();
        item(playback, "Sound effects", MENU_TOGGLE_MUTE, !cfg.mute);
        let speed = CreatePopupMenu();
        for (label, command, value) in [
            ("25%", MENU_SPEED_25, 25),
            ("50%", MENU_SPEED_50, 50),
            ("100%", MENU_SPEED_100, 100),
            ("150%", MENU_SPEED_150, 150),
            ("200%", MENU_SPEED_200, 200),
            ("300%", MENU_SPEED_300, 300),
            ("400%", MENU_SPEED_400, 400),
        ] {
            item(speed, label, command, cfg.speed == value);
        }
        submenu(playback, "Speed", speed, true);
        separator(playback);
        item(playback, "Show intro screen", MENU_TOGGLE_INTRO, cfg.intro);
        let transition = CreatePopupMenu();
        item(
            transition,
            "Faithful hard cuts",
            MENU_TRANSITION_CUT,
            cfg.transition == config::Transition::None,
        );
        item(
            transition,
            "Optional tiled dissolve",
            MENU_TRANSITION_DISSOLVE,
            cfg.transition == config::Transition::Dissolve,
        );
        submenu(playback, "Scene transition", transition, true);
        submenu(bar, "Playback", playback, true);

        let about = CreatePopupMenu();
        item(about, "About Castaway Continuum", MENU_ABOUT, false);
        submenu(bar, "About", about, true);
        if SetMenu(hwnd, bar) != 0 {
            DISPLAY_MENU_HANDLE.store(display, Ordering::Release);
            let _ = DrawMenuBar(hwnd);
        }
    }
}

/// Update the Classic/Custom checkmarks without replacing the window's menu
/// while the live preview handler is rebuilding the animation runtime.
#[cfg(windows)]
fn refresh_profile_menu_checks(window: &winit::window::Window, cfg: &config::Config) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    unsafe extern "system" {
        fn CheckMenuItem(menu: isize, item: u32, flags: u32) -> u32;
        fn DrawMenuBar(hwnd: isize) -> i32;
        fn GetMenu(hwnd: isize) -> isize;
    }
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = handle.hwnd.get();
    // SAFETY: hwnd belongs to this winit window and CheckMenuItem only changes
    // flags on its already-installed menu items.
    unsafe {
        let menu = GetMenu(hwnd);
        if menu == 0 {
            return;
        }
        const MF_BYCOMMAND: u32 = 0;
        const MF_CHECKED: u32 = 0x0008;
        let custom = MF_BYCOMMAND | if cfg.custom_enabled { MF_CHECKED } else { 0 };
        let original = MF_BYCOMMAND | if cfg.custom_enabled { 0 } else { MF_CHECKED };
        let _ = CheckMenuItem(menu, MENU_PROFILE_CUSTOM as u32, custom);
        let _ = CheckMenuItem(menu, MENU_PROFILE_ORIGINAL as u32, original);
        let _ = DrawMenuBar(hwnd);
    }
}

#[cfg(not(windows))]
fn install_native_menu(_: &winit::window::Window, _: &config::Config) {}

#[cfg(windows)]
fn apply_native_menu_command(cfg: &mut config::Config, command: u16) -> bool {
    if !cfg.custom_enabled && is_custom_menu_command(usize::from(command)) {
        return false;
    }
    match usize::from(command) {
        MENU_TOGGLE_ALWAYS_ON_TOP => cfg.always_on_top = !cfg.always_on_top,
        MENU_TOGGLE_IGNORE_MOUSE_CLOSE => cfg.ignore_mouse_close = !cfg.ignore_mouse_close,
        MENU_TOGGLE_FULL_SIZE_WINDOW => cfg.full_size_window = !cfg.full_size_window,
        MENU_RESET_CUSTOM => cfg.reset_custom_values(),
        MENU_CLEAR_FAVORITES => cfg.favorite_scenes.clear(),
        MENU_TOGGLE_ISLAND_LOCATION => {
            cfg.test_offset_enabled = !cfg.test_offset_enabled;
            // Island placement is a Custom setting, so enabling it should
            // also make the saved Custom profile active.
            cfg.custom_enabled = true;
        }
        MENU_PROFILE_CUSTOM => cfg.custom_enabled = true,
        MENU_PROFILE_ORIGINAL => cfg.custom_enabled = false,
        MENU_TOGGLE_RETURN_TO_PRIMARY => cfg.return_to_primary = !cfg.return_to_primary,
        MENU_TOGGLE_DEDITHER => cfg.dedither = !cfg.dedither,
        MENU_CRT_SCANLINES_OFF => cfg.crt_scanlines = config::CtrScanlines::Off,
        MENU_CRT_SCANLINES_LIGHT => cfg.crt_scanlines = config::CtrScanlines::Light,
        MENU_CRT_SCANLINES_MEDIUM => cfg.crt_scanlines = config::CtrScanlines::Medium,
        MENU_CRT_SCANLINES_STRONG => cfg.crt_scanlines = config::CtrScanlines::Strong,
        MENU_CRT_CURVATURE_OFF => cfg.crt_curvature = config::CrtCurvature::Off,
        MENU_CRT_CURVATURE_GENTLE => cfg.crt_curvature = config::CrtCurvature::Gentle,
        MENU_CRT_CURVATURE_CURVED => cfg.crt_curvature = config::CrtCurvature::Curved,
        MENU_CRT_CURVATURE_STRONG => cfg.crt_curvature = config::CrtCurvature::Strong,
        MENU_CRT_PHOSPHOR_MASK_OFF => cfg.crt_phosphor_mask = config::CrtPhosphorMask::Off,
        MENU_CRT_PHOSPHOR_MASK_SUBTLE => cfg.crt_phosphor_mask = config::CrtPhosphorMask::Subtle,
        MENU_CRT_PHOSPHOR_MASK_VISIBLE => cfg.crt_phosphor_mask = config::CrtPhosphorMask::Visible,
        MENU_CRT_BLOOM_OFF => cfg.crt_bloom = config::CrtBloom::Off,
        MENU_CRT_BLOOM_SUBTLE => cfg.crt_bloom = config::CrtBloom::Subtle,
        MENU_CRT_COLOR_NEUTRAL => cfg.crt_color_profile = config::CrtColorProfile::Neutral,
        MENU_CRT_COLOR_CLASSIC => cfg.crt_color_profile = config::CrtColorProfile::Classic,
        MENU_CRT_COLOR_WARM => cfg.crt_color_profile = config::CrtColorProfile::Warm,
        MENU_CRT_SOFTNESS_OFF => cfg.crt_horizontal_softness = config::CrtHorizontalSoftness::Off,
        MENU_CRT_SOFTNESS_LIGHT => {
            cfg.crt_horizontal_softness = config::CrtHorizontalSoftness::Light
        }
        MENU_CRT_SOFTNESS_CLASSIC => {
            cfg.crt_horizontal_softness = config::CrtHorizontalSoftness::Classic
        }
        MENU_TOGGLE_SCENE_INFO => cfg.scene_info_overlay = !cfg.scene_info_overlay,
        MENU_TOGGLE_SCENE_INFO_LONG_DESCRIPTION => {
            cfg.scene_info_long_description = !cfg.scene_info_long_description
        }
        MENU_SCENE_INFO_TOP_LEFT => cfg.scene_info_corner = config::SceneInfoCorner::TopLeft,
        MENU_SCENE_INFO_TOP_RIGHT => cfg.scene_info_corner = config::SceneInfoCorner::TopRight,
        MENU_SCENE_INFO_BOTTOM_LEFT => cfg.scene_info_corner = config::SceneInfoCorner::BottomLeft,
        MENU_SCENE_INFO_BOTTOM_RIGHT => {
            cfg.scene_info_corner = config::SceneInfoCorner::BottomRight
        }
        MENU_SCALE_FIT => cfg.scale = scale::ScaleMode::Fit,
        MENU_SCALE_STRETCH => cfg.scale = scale::ScaleMode::Stretch,
        MENU_SCALE_INTEGER => cfg.scale = scale::ScaleMode::Integer,
        MENU_SCALE_EXTEND => cfg.scale = scale::ScaleMode::Extend,
        MENU_FILTER_NEAREST => cfg.filter = scale::Filter::Nearest,
        MENU_FILTER_LINEAR => cfg.filter = scale::Filter::Linear,
        MENU_FILTER_XBR => cfg.filter = scale::Filter::Xbr,
        MENU_FILTER_XBRZ => cfg.filter = scale::Filter::Xbrz,
        MENU_LIGHTING_AUTHENTIC => {
            cfg.test_time = config::TestTime::Any;
            cfg.daynight = DayNight::Original;
        }
        MENU_LIGHTING_PC_TIME => {
            cfg.test_time = config::TestTime::Any;
            cfg.daynight = DayNight::Real24h;
        }
        MENU_LIGHTING_DAY => cfg.test_time = config::TestTime::Day,
        MENU_LIGHTING_NIGHT => cfg.test_time = config::TestTime::Night,
        MENU_TIDE_AUTHENTIC => cfg.tide_preference = config::TidePreference::Authentic,
        MENU_TIDE_HIGH => cfg.tide_preference = config::TidePreference::PreferHigh,
        MENU_TIDE_LOW => cfg.tide_preference = config::TidePreference::PreferLow,
        MENU_HOLIDAY_CALENDAR => cfg.test_holiday = config::TestHoliday::Calendar,
        MENU_HOLIDAY_OFF => cfg.test_holiday = config::TestHoliday::None,
        MENU_HOLIDAY_HALLOWEEN => cfg.test_holiday = config::TestHoliday::Halloween,
        MENU_HOLIDAY_ST_PATRICK => cfg.test_holiday = config::TestHoliday::StPatrick,
        MENU_HOLIDAY_CHRISTMAS => cfg.test_holiday = config::TestHoliday::Christmas,
        MENU_HOLIDAY_NEW_YEAR => cfg.test_holiday = config::TestHoliday::NewYear,
        MENU_TOGGLE_MUTE => cfg.mute = !cfg.mute,
        MENU_SPEED_25 => cfg.speed = 25,
        MENU_SPEED_50 => cfg.speed = 50,
        MENU_SPEED_100 => cfg.speed = 100,
        MENU_SPEED_150 => cfg.speed = 150,
        MENU_SPEED_200 => cfg.speed = 200,
        MENU_SPEED_300 => cfg.speed = 300,
        MENU_SPEED_400 => cfg.speed = 400,
        MENU_TOGGLE_INTRO => cfg.intro = !cfg.intro,
        MENU_TRANSITION_CUT => cfg.transition = config::Transition::None,
        MENU_TRANSITION_DISSOLVE => cfg.transition = config::Transition::Dissolve,
        _ => return false,
    }
    if matches!(
        usize::from(command),
        MENU_LIGHTING_AUTHENTIC
            | MENU_LIGHTING_PC_TIME
            | MENU_LIGHTING_DAY
            | MENU_LIGHTING_NIGHT
            | MENU_TIDE_AUTHENTIC
            | MENU_TIDE_HIGH
            | MENU_TIDE_LOW
            | MENU_HOLIDAY_CALENDAR
            | MENU_HOLIDAY_OFF
            | MENU_HOLIDAY_HALLOWEEN
            | MENU_HOLIDAY_ST_PATRICK
            | MENU_HOLIDAY_CHRISTMAS
            | MENU_HOLIDAY_NEW_YEAR
    ) {
        // An environment override is always a Custom action, even when the
        // prior profile was Original.
        cfg.custom_enabled = true;
    }
    true
}

#[cfg(windows)]
fn open_custom_dialog(custom: &mut Option<Child>) {
    if custom
        .as_mut()
        .is_some_and(|child| child.try_wait().ok().flatten().is_none())
    {
        focus_custom_dialog();
        return;
    }
    *custom = None;
    match std::env::current_exe() {
        Ok(exe) => match std::process::Command::new(exe)
            .arg("/c")
            .env("CASTAWAY_CONTINUUM_CUSTOM_DIALOG", "1")
            // The controller has its own persisted geometry. Explicitly set
            // the first tab so an old configuration-dialog state cannot make
            // the initial Custom window look empty or unrelated.
            .env("CASTAWAY_CONTINUUM_SETTINGS_TAB", "0")
            .spawn()
        {
            Ok(child) => *custom = Some(child),
            Err(error) => eprintln!("[castaway-continuum] Could not open Custom Settings: {error}"),
        },
        Err(error) => {
            eprintln!("[castaway-continuum] Could not locate Custom Settings executable: {error}")
        }
    }
}

/// Open the compact island-position controller, restoring the existing one
/// rather than spawning a stack of duplicate floating bars.
#[cfg(windows)]
fn open_island_location_bar(bar: &mut Option<Child>) {
    if bar
        .as_mut()
        .is_some_and(|child| child.try_wait().ok().flatten().is_none())
    {
        focus_island_location_bar();
        return;
    }
    *bar = None;
    if let Ok(exe) = std::env::current_exe() {
        *bar = std::process::Command::new(exe)
            .arg("/c")
            .env("CASTAWAY_CONTINUUM_ISLAND_LOCATION_BAR", "1")
            .spawn()
            .ok();
    }
}

/// Restore the one floating Custom dialog and place it in front of the display.
/// The process is already tracked above; the title lookup is only used to obtain
/// its native window handle so a minimized dialog can be restored.
#[cfg(windows)]
fn focus_custom_dialog() {
    unsafe extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn ShowWindow(hwnd: isize, command: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn BringWindowToTop(hwnd: isize) -> i32;
    }
    const SW_RESTORE: i32 = 9;
    let title = "Custom Settings"
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: a null class name asks Windows to match any top-level window class;
    // the UTF-16 title is nul-terminated and lives through these calls.
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
fn focus_island_location_bar() {
    unsafe extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn ShowWindow(hwnd: isize, command: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn BringWindowToTop(hwnd: isize) -> i32;
    }
    let title = "Island location"
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: we only operate on the controller window identified by its
    // fixed title; no pointers are retained beyond these calls.
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            const SW_RESTORE: i32 = 9;
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
fn close_custom_dialog(custom: &mut Option<Child>) {
    if let Some(mut child) = custom.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(windows)]
fn show_native_about(window: &winit::window::Window) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    unsafe extern "system" {
        fn MessageBoxW(hwnd: isize, text: *const u16, caption: *const u16, flags: u32) -> i32;
    }
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let text: Vec<u16> = format!(
        "Castaway Continuum {}\\nJohnny Castaway restoration and testing build.\\n\\n{}\\n\\nForked from Wilson Reborn. Original artwork and data: Sierra On-Line / Dynamix.\\nEngine: GPL-3.0-or-later.",
        env!("CARGO_PKG_VERSION"),
        stats::Stats::load().summary(),
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let caption: Vec<u16> = "About Castaway Continuum"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: hwnd is a live Win32 window and both strings remain NUL-terminated
    // for the duration of the synchronous MessageBoxW call.
    unsafe {
        let _ = MessageBoxW(handle.hwnd.get(), text.as_ptr(), caption.as_ptr(), 0x40);
    }
}

#[cfg(windows)]
fn confirm_reset_custom_values(window: &winit::window::Window) -> bool {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    unsafe extern "system" {
        fn MessageBoxW(hwnd: isize, text: *const u16, caption: *const u16, flags: u32) -> i32;
    }
    let Ok(handle) = window.window_handle() else {
        return false;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return false;
    };
    let text: Vec<u16> = "Reset all Custom scene, environment, holiday, playback, and saved island-location values?\n\nWindow, display, audio, and scale preferences will be kept."
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let caption: Vec<u16> = "Reset custom values"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: hwnd is a live Win32 window and the strings remain valid for
    // the synchronous prompt. IDYES is 6.
    unsafe { MessageBoxW(handle.hwnd.get(), text.as_ptr(), caption.as_ptr(), 0x24) == 6 }
}

/// Ask before removing a user's personally curated scene list. This is kept
/// separate from Reset Custom Values because Favorites are useful enough not
/// to make people rebuild them after one overzealous reset.
#[cfg(windows)]
fn confirm_clear_favorites(window: &winit::window::Window) -> bool {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    unsafe extern "system" {
        fn MessageBoxW(hwnd: isize, text: *const u16, caption: *const u16, flags: u32) -> i32;
    }
    let Ok(handle) = window.window_handle() else {
        return false;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return false;
    };
    let text: Vec<u16> = "Clear every scene from Favorites?\n\nThis only removes the Favorites list. Your other Custom settings will be kept."
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let caption: Vec<u16> = "Clear favorites".encode_utf16().chain(Some(0)).collect();
    // IDYES is 6. The strings live through the synchronous native prompt.
    unsafe { MessageBoxW(handle.hwnd.get(), text.as_ptr(), caption.as_ptr(), 0x24) == 6 }
}

/// Transient geometry passed only by the local Save & Preview handoff. It is
/// deliberately not persisted: normal launches retain the user's normal
/// startup placement rules.
fn preview_geometry_from_env() -> Option<(
    winit::dpi::PhysicalPosition<i32>,
    winit::dpi::PhysicalSize<u32>,
)> {
    let values: Vec<_> = std::env::var("WILSON_PREVIEW_GEOMETRY")
        .ok()?
        .split(',')
        .map(str::parse::<i32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let [x, y, width, height] = values.as_slice() else {
        return None;
    };
    let (width, height) = (u32::try_from(*width).ok()?, u32::try_from(*height).ok()?);
    if width < 640 || height < 480 {
        return None;
    }
    Some((
        winit::dpi::PhysicalPosition::new(*x, *y),
        winit::dpi::PhysicalSize::new(width, height),
    ))
}

/// Bind Wilson's loopback control socket. A Save & Preview replacement waits
/// for the previous Wilson process to relinquish it before showing a window.
/// That turns the handoff into a serial replacement instead of two savers
/// briefly (or occasionally persistently) competing on the desktop.
fn bind_preview_socket(wait_for_previous: bool) -> Option<UdpSocket> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match UdpSocket::bind("127.0.0.1:42781") {
            Ok(socket) => {
                let _ = socket.set_nonblocking(true);
                return Some(socket);
            }
            Err(_) if wait_for_previous && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return None,
        }
    }
}

/// Decode the transient choices sent by the Settings panel to a running
/// Wilson window. Nothing here is persisted unless the panel separately saves.
fn parse_preview_options(
    raw: &str,
) -> Option<(u8, config::TestTime, bool, i32, i32, config::TidePreference)> {
    let parts: Vec<_> = raw.split('|').collect();
    let [scene, time, offset_enabled, offset_x, offset_y, tide] = parts.as_slice() else {
        return None;
    };
    let (Ok(scene), Some(time), Ok(offset_enabled), Ok(offset_x), Ok(offset_y), Some(tide)) = (
        scene.parse::<u8>(),
        match *time {
            "any" => Some(config::TestTime::Any),
            "day" => Some(config::TestTime::Day),
            "night" => Some(config::TestTime::Night),
            _ => None,
        },
        offset_enabled.parse::<bool>(),
        offset_x.parse::<i32>(),
        offset_y.parse::<i32>(),
        match *tide {
            "authentic" => Some(config::TidePreference::Authentic),
            "high" => Some(config::TidePreference::PreferHigh),
            "low" => Some(config::TidePreference::PreferLow),
            _ => None,
        },
    ) else {
        return None;
    };
    Some((
        scene.min(63),
        time,
        offset_enabled,
        offset_x,
        offset_y,
        tide,
    ))
}

/// `Calendar` preserves the original date logic; every other selection is an
/// explicit testing override and therefore works on any day of the year.
fn test_holiday_override(holiday: config::TestHoliday) -> Option<Holiday> {
    match holiday {
        config::TestHoliday::Calendar => None,
        config::TestHoliday::None => Some(Holiday::None),
        config::TestHoliday::Halloween => Some(Holiday::Halloween),
        config::TestHoliday::StPatrick => Some(Holiday::StPatrick),
        config::TestHoliday::Christmas => Some(Holiday::Christmas),
        config::TestHoliday::NewYear => Some(Holiday::NewYear),
    }
}

/// Apply the Custom display manager's playback selection.  Group playback starts
/// from the browser selection; repeating remains an explicit, separate state.
fn enable_configured_test_playback(show: &mut Show, archive: &Archive, cfg: &config::Config) {
    if !cfg.custom_enabled {
        return;
    }
    if cfg.test_scene != 0 {
        let offset = cfg
            .test_offset_enabled
            .then_some((cfg.test_offset_x, cfg.test_offset_y));
        show.enable_scene_test_repeat(archive, usize::from(cfg.test_scene - 1), offset);
        return;
    }

    let start_index = cfg.test_group_start.checked_sub(1).map(usize::from);
    match cfg.test_group {
        config::TestGroup::Off => {}
        config::TestGroup::Special => {
            let start_day = start_index
                .and_then(|index| STORY_SCENES.get(index))
                .map(|scene| scene.day)
                .filter(|&day| day != 0);
            show.enable_special_cycle_days_from(archive, &cfg.special_days, start_day);
        }
        config::TestGroup::Visitors => {
            show.enable_scene_test_cycle_from_scene(archive, TestSceneGroup::Visitors, start_index)
        }
        config::TestGroup::Normal => {
            show.enable_scene_test_cycle_from_scene(archive, TestSceneGroup::Normal, start_index)
        }
        config::TestGroup::Common => {
            show.enable_scene_test_cycle_from_scene(archive, TestSceneGroup::Common, start_index)
        }
        config::TestGroup::Uncommon => {
            show.enable_scene_test_cycle_from_scene(archive, TestSceneGroup::Uncommon, start_index)
        }
        config::TestGroup::All => {
            show.enable_scene_test_cycle_from_scene(archive, TestSceneGroup::All, start_index)
        }
        config::TestGroup::Favorites => {
            let favorites: Vec<usize> = cfg
                .favorite_scenes
                .iter()
                .map(|&scene| usize::from(scene.saturating_sub(1)))
                .collect();
            if !favorites.is_empty() {
                show.enable_scene_test_favorites(archive, &favorites, start_index);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Help first, so `-h`/`--help` (Unix) and `/?`/`/help` (Windows) always work and
    // never touch the data/window.
    if wants_help(&args) {
        print_help();
        return;
    }

    // Load options (file → defaults), seed a default file on first run, then let CLI
    // flags override for this run only.
    let mut cfg = config::Config::load();
    for warning in &cfg.catalog_warnings {
        eprintln!("[wilson] Scene catalog: {warning}");
    }
    if config::config_file().is_some_and(|p| !p.exists()) {
        save_config_or_report(&cfg);
    }
    cfg.apply_args(&args);

    // Windows screensaver verbs.
    let action = screensaver_action(&args);
    match action {
        // The Windows Screen Saver control panel sends `/c`. Launch the
        // sibling normal EXE in windowed mode so it provides the full live
        // display and existing menus rather than a cramped, incomplete clone
        // of the configuration interface.
        Action::Configure if std::env::var_os("CASTAWAY_CONTINUUM_CUSTOM_DIALOG").is_some() => {
            settings_ui::open(cfg.clone());
            return;
        }
        Action::Configure => {
            if let Err(error) = launch_sibling_configuration_app() {
                eprintln!("[castaway-continuum] {error}");
            }
            return;
        }
        Action::Preview(_) if !cfg!(windows) => {
            eprintln!("Castaway Continuum: the /p preview is only supported on Windows.");
            return;
        }
        _ => {}
    }
    let preview_parent: Option<isize> = match action {
        Action::Preview(hwnd) => Some(hwnd),
        _ => None,
    };
    let is_preview = preview_parent.is_some();
    // Only `/s` is a real Windows screensaver session. A plain EXE launch keeps
    // the application's useful click-to-toggle/full menu behavior; `/s` must
    // always be borderless fullscreen and must immediately dismiss on input.
    let is_screensaver = matches!(action, Action::Show);
    if is_screensaver {
        cfg.windowed = false;
        cfg.full_size_window = false;
        cfg.always_on_top = false;
    }

    // Original data + sounds: embedded (self-contained `embed-data` build) or loaded
    // from disk (`--data`/auto-detect).
    #[cfg(feature = "embed-data")]
    let (archive, palette, mut audio) = {
        let (archive, palette) = embedded::archive_and_palette();
        let audio =
            audio::Audio::from_sounds(embedded::sound_bytes(), audio_muted(cfg.mute, is_preview));
        (archive, palette, audio)
    };
    #[cfg(not(feature = "embed-data"))]
    let (archive, palette, mut audio) = {
        let data_arg = args
            .windows(2)
            .find(|w| w[0] == "--data")
            .map(|w| w[1].clone());
        // Accept a folder OR a .zip (e.g. the Internet Archive scrantic-run.zip), and
        // auto-detect either next to the executable / in the working directory.
        let data_dir = match assets::resolve_data_dir(data_arg.as_deref()) {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        let (archive, palette) = match assets::load(&data_dir) {
            Ok(loaded) => loaded,
            Err(e) => {
                eprintln!("Could not load data from {}: {e}", data_dir.display());
                return;
            }
        };
        // Sound effects: soundN.wav in the data dir, else extracted from SCRANTIC.EXE/.SCR
        // there. Degrades to silence without the `audio` feature, a device, or mute.
        let audio = audio::Audio::new(Some(data_dir.as_path()), audio_muted(cfg.mute, is_preview));
        (archive, palette, audio)
    };

    let real = clock::now();
    // Time controls (QoL, opt-in): `--story` plays the whole arc in order from a synthetic
    // clock; `--day N` starts at a chosen day. Both are one-off — in those modes we do NOT
    // persist the day, so the real saved arc isn't clobbered. Default: resume the persisted
    // day (or today), real wall-clock time, and apply the configured day/night cycle.
    let persist_day = !cfg.story
        && (!cfg.custom_enabled || cfg.test_group == config::TestGroup::Off)
        && cfg.day == 0;
    let (director, mut clock) = if cfg.story {
        // Day 1, stored_yday 0: the story clock (day-index 0 at t=0) keeps the first run on
        // day 1, then advances the arc as its day-index ticks.
        (
            Director::new(1, 0),
            timectl::story_clock(real, 0, cfg.story_secs),
        )
    } else if cfg.day != 0 {
        (Director::new(cfg.day, real.yday), real)
    } else {
        let d = match state::DayState::load() {
            Some(s) => Director::new(s.current_day, s.stored_yday),
            None => Director::new(1, real.yday),
        };
        (d, real)
    };
    let director = director.with_daynight(cfg.daynight);
    // Testing needs a repeatable lighting state, not a hostage situation where the
    // tester waits for the wall clock. Choose representative hours for either cycle.
    if cfg.custom_enabled && cfg.test_time != config::TestTime::Any {
        clock.hour = match (cfg.test_time, cfg.daynight) {
            (config::TestTime::Any, _) => clock.hour,
            (config::TestTime::Day, DayNight::Original) => 1,
            (config::TestTime::Night, DayNight::Original) => 0,
            (config::TestTime::Day, DayNight::Real24h) => 12,
            (config::TestTime::Night, DayNight::Real24h) => 23,
        };
    }
    let mut seed = fresh_run_seed();
    let mut show = Show::new(&archive, &palette, 640, 480, director, clock, seed);
    show.set_holiday_animations(&archive, cfg.holiday_animations);
    show.set_holiday_override(
        &archive,
        cfg.custom_enabled
            .then(|| test_holiday_override(cfg.test_holiday))
            .flatten(),
    );
    show.set_tide_preference(
        &archive,
        match if cfg.custom_enabled {
            cfg.tide_preference
        } else {
            config::TidePreference::Authentic
        } {
            config::TidePreference::Authentic => TidePreference::Authentic,
            config::TidePreference::PreferHigh => TidePreference::PreferHigh,
            config::TidePreference::PreferLow => TidePreference::PreferLow,
        },
    );
    enable_configured_test_playback(&mut show, &archive, &cfg);
    show.set_island_offset_override(
        &archive,
        (cfg.custom_enabled && cfg.test_offset_enabled)
            .then_some((cfg.test_offset_x, cfg.test_offset_y)),
    );
    if cfg.intro {
        // the original's startup intro screen (INTRO.SCR), held for the configured seconds
        show.enable_intro(
            &archive,
            wilson_engine::intro_ticks_from_secs(cfg.intro_secs),
        );
    }
    if cfg.transition == config::Transition::Dissolve {
        show.enable_dissolve(); // the original's dormant LFSR tiled dissolve, opt-in
    }

    // Persist the story day whenever it advances, so the arc carries over to the next
    // run. `None` until the first frame establishes today's day.
    let mut last_saved: Option<(u8, i32)> = None;

    // Lifetime statistics: count this session and accumulate runtime.
    let mut stats = stats::Stats::load();
    stats.sessions += 1;
    stats.note_day(show.day_state().0);
    report_persistence_error(stats.save());
    let base_secs = stats.total_secs;
    let session_start = Instant::now();
    let mut last_flush = Instant::now();
    // Pace the engine off a per-frame deadline so spurious OS redraws (the startup burst,
    // resizes, scale changes) can't race past the timer and play the first frames — most
    // visibly the intro — too fast. `last_rgba` lets a non-due redraw repaint without
    // stepping the animation. See `pace::FramePacer`.
    let mut pacer = pace::FramePacer::new();
    let mut last_rgba: Option<Vec<u8>> = None;
    // Waves are a separate original timer: actor scripts may hold a frame longer
    // than one wave phase, so they need their own deadline.
    let mut wave_period = Duration::from_millis(cfg.frame_delay_ms(WAVE_PERIOD_TICKS));
    let mut next_wave_due: Option<Instant> = None;

    // --debug diagnostics: measured FPS over a rolling 1-second window.
    let mut dbg_frames = 0u32;
    let mut dbg_fps = 0u32;
    let mut dbg_window = Instant::now();
    let mut dbg_first_frame = true; // log the first successful frame once
                                    // Some stage transitions deliberately have no active ADS for a handful
                                    // of frames. Keep the most recent identified scene for the friendly
                                    // overlay so the label does not blink like a cheap haunted sign.
    let mut last_scene_info: Option<wilson_engine::DebugInfo> = None;
    if cfg.debug {
        eprintln!(
            "[wilson:debug] on — filter={} scale={} speed={}% dedither={} daynight={} windowed={} intro={}",
            cfg.filter.as_str(),
            cfg.scale.as_str(),
            cfg.speed,
            cfg.dedither,
            cfg.daynight.as_str(),
            cfg.windowed,
            cfg.intro,
        );
        eprintln!("[wilson:debug] audio: {}", audio.debug_summary());
    }

    let event_loop = {
        let mut builder = EventLoopBuilder::new();
        #[cfg(windows)]
        {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            builder.with_msg_hook(|raw| {
                // SAFETY: winit passes a valid MSG pointer for the duration of
                // this hook. We only read the WM_COMMAND fields.
                let message = unsafe { &*(raw as *const NativeMessage) };
                if message.message != 0x0111 {
                    return false;
                }
                let command = message.w_param & 0xFFFF;
                if !(MENU_EXIT..=MENU_ABOUT).contains(&command) {
                    return false;
                }
                NATIVE_MENU_COMMAND.store(command as u16, Ordering::Release);
                true
            });
        }
        builder.build().expect("failed to create event loop")
    };
    // Loopback-only handoff used by the settings panel's Save & Preview button.
    // This keeps the implementation portable to Linux/Raspberry Pi too.
    let replacement = std::env::var_os("WILSON_PREVIEW_REPLACEMENT").is_some();
    let preview_reload_socket = bind_preview_socket(replacement);
    if replacement && preview_reload_socket.is_none() {
        // A prior process did not yield within the bounded handoff window.
        // Refuse to open a second saver on top of it.
        return;
    }
    let preview_geometry = preview_geometry_from_env();
    let mut builder = WindowBuilder::new().with_title(format!(
        "Castaway Continuum {} - Johnny Castaway",
        env!("CARGO_PKG_VERSION")
    ));
    builder = if let Some(hwnd) = preview_parent {
        apply_preview(builder, hwnd) // embedded preview pane (Windows /p)
    } else if cfg.windowed {
        // The original art is a 640x480 (4:3) pixel canvas. Keep that as the
        // minimum client size so it is never fractionally reduced into a
        // dither/moiré grid, while retaining standard resize/maximize controls.
        // `ScaleMode::Fit` preserves 4:3 and letterboxes larger windows.
        let builder = builder
            .with_inner_size(winit::dpi::PhysicalSize::new(640, 480))
            .with_min_inner_size(winit::dpi::PhysicalSize::new(640, 480));
        if cfg.full_size_window {
            builder.with_maximized(true)
        } else {
            builder
        }
    } else {
        builder.with_fullscreen(Some(Fullscreen::Borderless(None)))
    };
    if cfg.windowed {
        if let Some((position, size)) = preview_geometry {
            builder = builder.with_position(position).with_inner_size(size);
        }
    }
    let window = Rc::new(builder.build(&event_loop).expect("failed to create window"));
    let mut runtime_windowed = cfg.windowed;
    #[cfg(windows)]
    let mut native_menu_installed = false;
    if cfg.windowed {
        install_native_menu(&window, &cfg);
        #[cfg(windows)]
        {
            native_menu_installed = true;
        }
        // SetMenu adds non-client/menu chrome after winit created the window.
        // Ask Windows for the original 640x480 *client* canvas again.
        let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(640, 480));
    }
    if cfg.windowed && cfg.always_on_top {
        window.set_window_level(WindowLevel::AlwaysOnTop);
    }
    if cfg.debug {
        let s = window.inner_size();
        let mode = if is_preview {
            "preview"
        } else if cfg.windowed {
            "windowed"
        } else {
            "fullscreen"
        };
        eprintln!(
            "[wilson:debug] window built ({mode}) {}x{}; entering event loop",
            s.width, s.height
        );
    }
    let context = softbuffer::Context::new(window.clone()).expect("softbuffer context");
    let mut surface =
        softbuffer::Surface::new(&context, window.clone()).expect("softbuffer surface");
    // Explorer's double-click can arrive after the window is created. Without a short
    // grace period it is mistaken for screensaver-dismiss input and the app appears
    // to "not open" at all.
    let input_grace_until = Instant::now() + Duration::from_millis(750);
    let mut modifiers = ModifiersState::empty();
    #[cfg(windows)]
    let mut custom_dialog: Option<Child> = None;
    #[cfg(windows)]
    let mut island_location_bar: Option<Child> = None;
    // Preserve the exact framed geometry before a click sends the display fullscreen.
    // A maximized default intentionally comes back maximized instead.
    let mut restore_window_geometry = None;

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, .. } => {
                // --debug: trace window events (skip the per-frame/noisy ones) so a quick
                // exit shows *which* event caused it (e.g. an input event closing it).
                if cfg.debug
                    && !matches!(
                        event,
                        WindowEvent::RedrawRequested
                            | WindowEvent::CursorMoved { .. }
                            | WindowEvent::AxisMotion { .. }
                            | WindowEvent::Moved(_)
                    )
                {
                    eprintln!("[wilson:debug] window event: {event:?}");
                }
                match event {
                    WindowEvent::ModifiersChanged(change) => modifiers = change.state(),
                    WindowEvent::KeyboardInput { event: key, .. }
                        if key.state == ElementState::Pressed
                            && modifiers.control_key()
                            && modifiers.alt_key()
                            && matches!(key.physical_key, PhysicalKey::Code(KeyCode::KeyJ)) =>
                    {
                        if modifiers.shift_key() {
                            elwt.exit();
                        } else if let Ok(exe) = std::env::current_exe() {
                            // The same Custom controller is available on Windows and the
                            // Raspberry Pi/Linux build; Windows' Display menu is merely an
                            // additional way to open it.
                            let _ = std::process::Command::new(exe)
                                .arg("/c")
                                .env("CASTAWAY_CONTINUUM_CUSTOM_DIALOG", "1")
                                .spawn();
                        }
                    }
                    // The preview pane must keep running on hover/keypress; only a real
                    // close ends it. Normal display clicks toggle framed/fullscreen modes.
                    WindowEvent::CloseRequested => {
                        if cfg.debug {
                            eprintln!("[wilson:debug] exit: CloseRequested");
                        }
                        #[cfg(windows)]
                        close_custom_dialog(&mut custom_dialog);
                        #[cfg(windows)]
                        close_custom_dialog(&mut island_location_bar);
                        elwt.exit();
                    }
                    WindowEvent::CursorMoved { .. }
                        if is_screensaver && Instant::now() >= input_grace_until =>
                    {
                        if cfg.debug {
                            eprintln!("[wilson:debug] exit: CursorMoved (screensaver)");
                        }
                        elwt.exit();
                    }
                    WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state,
                        ..
                    }
                        if Instant::now() >= input_grace_until
                            && is_screensaver
                            && state == ElementState::Pressed =>
                    {
                        if cfg.debug {
                            eprintln!("[wilson:debug] exit: MouseInput (screensaver)");
                        }
                        elwt.exit();
                    }
                    WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state,
                        ..
                    }
                        if Instant::now() >= input_grace_until
                            && !is_preview
                            && !is_screensaver
                            && state == ElementState::Pressed =>
                    {
                        if cfg.debug {
                            eprintln!("[wilson:debug] toggle display mode: left MouseInput");
                        }
                        if runtime_windowed {
                            if !cfg.full_size_window {
                                if let Ok(position) = window.outer_position() {
                                    restore_window_geometry = Some((position, window.inner_size()));
                                }
                            }
                            // A topmost fullscreen saver would compete with Custom for the
                            // same Windows z-order level. Keep the saver normal while it is
                            // fullscreen so the explicitly topmost Custom dialog remains over
                            // it, then restore this preference when returning to a window.
                            window.set_window_level(WindowLevel::Normal);
                            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                            runtime_windowed = false;
                        } else {
                            window.set_fullscreen(None);
                            runtime_windowed = true;
                            #[cfg(windows)]
                            if !native_menu_installed {
                                install_native_menu(&window, &cfg);
                                native_menu_installed = true;
                            }
                            if cfg.full_size_window {
                                window.set_maximized(true);
                            } else if let Some((position, size)) = restore_window_geometry.take() {
                                window.set_maximized(false);
                                window.set_outer_position(position);
                                let _ = window.request_inner_size(size);
                            }
                            window.set_window_level(if cfg.always_on_top {
                                WindowLevel::AlwaysOnTop
                            } else {
                                WindowLevel::Normal
                            });
                        }
                    }
                    WindowEvent::KeyboardInput {
                        event: key,
                        is_synthetic,
                        ..
                    } if !is_preview
                        && Instant::now() >= input_grace_until
                        && key_dismisses(&key.logical_key, key.state, is_synthetic) =>
                    {
                        if cfg.debug {
                            eprintln!("[wilson:debug] exit: KeyboardInput {:?}", key.logical_key);
                        }
                        elwt.exit();
                    }
                    WindowEvent::RedrawRequested => {
                        let size = window.inner_size();
                        if let (Some(w), Some(h)) =
                            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                        {
                            // Pace from when the frame became due (≈ when the timer fired),
                            // not from after we finish computing it: this absorbs the per-frame
                            // work (engine + xBR/scale/present) into the delay instead of adding
                            // it on top, matching jc_reborn's `lastTicks` pacing. Without this,
                            // heavier filters (xBR) make the animation run slower than the
                            // original.
                            let frame_start = Instant::now();
                            surface.resize(w, h).expect("resize surface");
                            if pacer.due(frame_start) {
                                if next_wave_due.is_some_and(|due| frame_start >= due) {
                                    let _ = show.advance_background(WAVE_PERIOD_TICKS);
                                    next_wave_due = Some(frame_start + wave_period);
                                }
                                // The current frame's hold has elapsed (or this is the first
                                // frame): step the engine. Refresh the clock so the day rolls
                                // over within a long session — story mode feeds a synthetic
                                // clock (the arc in order); otherwise the real wall clock.
                                show.set_clock(if cfg.story {
                                    timectl::story_clock(
                                        clock::now(),
                                        session_start.elapsed().as_secs(),
                                        cfg.story_secs,
                                    )
                                } else {
                                    clock::now()
                                });
                                let frame = show.next_frame(&archive);
                                for &id in &frame.sounds {
                                    let outcome = audio.play(id);
                                    if cfg.debug {
                                        eprintln!("[wilson:debug] sound cue {id}: {outcome:?}");
                                    }
                                }
                                let (day, yday) = show.day_state();
                                // Persist the day only in real-time mode — `--story`/`--day`
                                // are one-off overrides and must not clobber the saved arc.
                                if persist_day && last_saved != Some((day, yday)) {
                                    report_persistence_error(
                                        state::DayState {
                                            current_day: day,
                                            stored_yday: yday,
                                        }
                                        .save(),
                                    );
                                    last_saved = Some((day, yday));
                                }
                                // Update lifetime stats, flushing to disk occasionally.
                                stats.note_day(day);
                                if last_flush.elapsed() >= Duration::from_secs(30) {
                                    stats.total_secs =
                                        base_secs + session_start.elapsed().as_secs();
                                    report_persistence_error(stats.save());
                                    last_flush = Instant::now();
                                }
                                let rgba = frame.surface.to_rgba(&palette);
                                // Optional: smooth the dithered sea/sky before scaling.
                                let rgba = if cfg.dedither {
                                    wilson_engine::dedither(&rgba, 640, 480)
                                } else {
                                    rgba
                                };
                                let runtime_info = show.debug_info();
                                if runtime_info.scene.is_some() {
                                    last_scene_info = Some(runtime_info.clone());
                                }
                                {
                                    let mut buffer = surface.buffer_mut().expect("surface buffer");
                                    scale::scale_rgba_to_argb(
                                        &rgba,
                                        640,
                                        480,
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.scale,
                                        cfg.filter,
                                    );
                                    if cfg.debug {
                                        // Measure FPS over a 1s window; emit a stdout status
                                        // line each second and draw the on-screen HUD.
                                        dbg_frames += 1;
                                        if dbg_window.elapsed() >= Duration::from_secs(1) {
                                            dbg_fps = dbg_frames;
                                            dbg_frames = 0;
                                            dbg_window = Instant::now();
                                            let d = show.debug_info();
                                            let scene = d
                                                .scene
                                                .map(|(n, t)| format!("{n}#{t}"))
                                                .unwrap_or_else(|| "-".into());
                                            let off = d
                                                .offset
                                                .map(|(x, y)| format!("{x},{y}"))
                                                .unwrap_or_else(|| "-".into());
                                            eprintln!(
                                                "[wilson:debug] fps={} delay={}t stage={} \
                                                 day={}/11 scene={} drift=({}) night={} \
                                                 tide={} raft={} holiday={:?}",
                                                dbg_fps,
                                                frame.delay_ticks,
                                                d.stage,
                                                d.day,
                                                scene,
                                                off,
                                                d.night as u8,
                                                d.low_tide as u8,
                                                d.raft,
                                                d.holiday,
                                            );
                                        }
                                        draw_debug_hud(
                                            &mut buffer,
                                            size.width as usize,
                                            size.height as usize,
                                            dbg_fps,
                                            frame.delay_ticks,
                                            &runtime_info,
                                            &cfg,
                                        );
                                    }
                                    if cfg.scene_info_overlay {
                                        if let Some(info) = last_scene_info.as_ref() {
                                            draw_scene_info_overlay(
                                                &mut buffer,
                                                size.width as usize,
                                                size.height as usize,
                                                info,
                                                &cfg,
                                                cfg.scene_info_corner,
                                                cfg.scene_info_long_description,
                                                cfg.crt_curvature,
                                                scale::content_rect(
                                                    640,
                                                    480,
                                                    size.width as usize,
                                                    size.height as usize,
                                                    cfg.scale,
                                                ),
                                            );
                                        }
                                    }
                                    apply_crt_curvature(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_curvature,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_bloom(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_bloom,
                                        cfg.crt_color_profile,
                                        cfg.crt_horizontal_softness,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_phosphor_mask(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_phosphor_mask,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_scanlines(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_scanlines,
                                    );
                                    buffer.present().expect("present");
                                }
                                if cfg.debug && dbg_first_frame {
                                    dbg_first_frame = false;
                                    eprintln!("[wilson:debug] first frame presented {w}x{h}");
                                }
                                let delay =
                                    Duration::from_millis(cfg.frame_delay_ms(frame.delay_ticks));
                                // Deadline from the frame's due time, so compute time is
                                // absorbed (period ≈ delay); if compute overran it is already
                                // in the past and the next frame runs immediately.
                                let due = pacer.schedule(frame_start, delay);
                                let wave_due =
                                    next_wave_due.get_or_insert(frame_start + wave_period);
                                elwt.set_control_flow(ControlFlow::WaitUntil(due.min(*wave_due)));
                                last_rgba = Some(rgba);
                            } else {
                                // A redraw arrived before the current frame's hold elapsed
                                // (the OS startup burst, a resize, a scale change). Re-present
                                // the last frame at the current size so resizes still repaint —
                                // but do NOT step the animation, or the intro/first frames flash
                                // by. Re-arm the existing deadline without moving it.
                                let redraw_background =
                                    next_wave_due.is_some_and(|due| frame_start >= due);
                                if redraw_background {
                                    next_wave_due = Some(frame_start + wave_period);
                                }
                                if let Some(background_surface) = redraw_background
                                    .then(|| show.advance_background(WAVE_PERIOD_TICKS))
                                    .flatten()
                                {
                                    let rgba = background_surface.to_rgba(&palette);
                                    let rgba = if cfg.dedither {
                                        wilson_engine::dedither(&rgba, 640, 480)
                                    } else {
                                        rgba
                                    };
                                    let mut buffer = surface.buffer_mut().expect("surface buffer");
                                    scale::scale_rgba_to_argb(
                                        &rgba,
                                        640,
                                        480,
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.scale,
                                        cfg.filter,
                                    );
                                    // A background-only repaint is still a full window
                                    // presentation. Redraw the persistent scene overlay too,
                                    // otherwise it alternates with bare frames and flickers.
                                    if cfg.scene_info_overlay {
                                        if let Some(info) = last_scene_info.as_ref() {
                                            draw_scene_info_overlay(
                                                &mut buffer,
                                                size.width as usize,
                                                size.height as usize,
                                                info,
                                                &cfg,
                                                cfg.scene_info_corner,
                                                cfg.scene_info_long_description,
                                                cfg.crt_curvature,
                                                scale::content_rect(
                                                    640,
                                                    480,
                                                    size.width as usize,
                                                    size.height as usize,
                                                    cfg.scale,
                                                ),
                                            );
                                        }
                                    }
                                    apply_crt_curvature(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_curvature,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_bloom(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_bloom,
                                        cfg.crt_color_profile,
                                        cfg.crt_horizontal_softness,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_phosphor_mask(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_phosphor_mask,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_scanlines(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_scanlines,
                                    );
                                    buffer.present().expect("present");
                                    last_rgba = Some(rgba);
                                } else if let Some(rgba) = &last_rgba {
                                    let mut buffer = surface.buffer_mut().expect("surface buffer");
                                    scale::scale_rgba_to_argb(
                                        rgba,
                                        640,
                                        480,
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.scale,
                                        cfg.filter,
                                    );
                                    // Same rule for ordinary re-presents: every buffer that
                                    // reaches the screen must receive the overlay.
                                    if cfg.scene_info_overlay {
                                        if let Some(info) = last_scene_info.as_ref() {
                                            draw_scene_info_overlay(
                                                &mut buffer,
                                                size.width as usize,
                                                size.height as usize,
                                                info,
                                                &cfg,
                                                cfg.scene_info_corner,
                                                cfg.scene_info_long_description,
                                                cfg.crt_curvature,
                                                scale::content_rect(
                                                    640,
                                                    480,
                                                    size.width as usize,
                                                    size.height as usize,
                                                    cfg.scale,
                                                ),
                                            );
                                        }
                                    }
                                    apply_crt_curvature(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_curvature,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_bloom(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_bloom,
                                        cfg.crt_color_profile,
                                        cfg.crt_horizontal_softness,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_phosphor_mask(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_phosphor_mask,
                                        scale::content_rect(
                                            640,
                                            480,
                                            size.width as usize,
                                            size.height as usize,
                                            cfg.scale,
                                        ),
                                    );
                                    apply_crt_scanlines(
                                        &mut buffer,
                                        size.width as usize,
                                        size.height as usize,
                                        cfg.crt_scanlines,
                                    );
                                    buffer.present().expect("present");
                                }
                                if let Some(actor_due) = pacer.deadline() {
                                    let due = next_wave_due
                                        .map_or(actor_due, |wave_due| actor_due.min(wave_due));
                                    elwt.set_control_flow(ControlFlow::WaitUntil(due));
                                }
                            }
                        } else if cfg.debug && dbg_first_frame {
                            dbg_first_frame = false;
                            eprintln!(
                                "[wilson:debug] redraw skipped: window size {}x{} (zero)",
                                size.width, size.height
                            );
                        }
                    }
                    _ => {}
                }
            }
            // Pace the animation: redraw only when the per-frame timer elapses (or on
            // the initial start), not on every loop iteration. Requesting a redraw on
            // every `AboutToWait` would preempt the `WaitUntil` deadline and run the
            // engine as fast as the CPU spins (the screensaver played far too fast).
            Event::NewEvents(cause @ (StartCause::Init | StartCause::ResumeTimeReached { .. })) => {
                #[cfg(windows)]
                let command = NATIVE_MENU_COMMAND.swap(0, Ordering::AcqRel);
                #[cfg(windows)]
                if usize::from(command) == MENU_EXIT {
                    elwt.exit();
                } else if usize::from(command) == MENU_CUSTOM {
                    open_custom_dialog(&mut custom_dialog);
                } else if usize::from(command) == MENU_ISLAND_LOCATION {
                    open_island_location_bar(&mut island_location_bar);
                } else if usize::from(command) == MENU_ABOUT {
                    show_native_about(&window);
                } else if usize::from(command) == MENU_RESET_CUSTOM
                    && !confirm_reset_custom_values(&window)
                {
                    // The user chose No; deliberately leave everything alone.
                } else if usize::from(command) == MENU_CLEAR_FAVORITES
                    && !confirm_clear_favorites(&window)
                {
                    // Same rule: a cancelled confirmation changes nothing.
                } else if command != 0 {
                    let was_custom_enabled = cfg.custom_enabled;
                    if !apply_native_menu_command(&mut cfg, command) {
                        return;
                    }
                    if usize::from(command) == MENU_PROFILE_ORIGINAL && was_custom_enabled {
                        // Start Default mode with a new original run instead of
                        // reconstructing the Custom-era run from the same seed.
                        seed = fresh_run_seed();
                    }
                    let rebuild_show = matches!(
                        usize::from(command),
                        MENU_PROFILE_CUSTOM
                            | MENU_PROFILE_ORIGINAL
                            | MENU_LIGHTING_AUTHENTIC
                            | MENU_LIGHTING_PC_TIME
                            | MENU_LIGHTING_DAY
                            | MENU_LIGHTING_NIGHT
                            | MENU_TIDE_AUTHENTIC
                            | MENU_TIDE_HIGH
                            | MENU_TIDE_LOW
                            | MENU_HOLIDAY_CALENDAR
                            | MENU_HOLIDAY_OFF
                            | MENU_HOLIDAY_HALLOWEEN
                            | MENU_HOLIDAY_ST_PATRICK
                            | MENU_HOLIDAY_CHRISTMAS
                            | MENU_HOLIDAY_NEW_YEAR
                            | MENU_TOGGLE_ISLAND_LOCATION
                            | MENU_RESET_CUSTOM
                            | MENU_CLEAR_FAVORITES
                    );
                    let full_size_window_changed =
                        usize::from(command) == MENU_TOGGLE_FULL_SIZE_WINDOW;
                    save_config_or_report(&cfg);
                    audio.set_muted(audio_muted(cfg.mute, is_preview));
                    if runtime_windowed {
                        window.set_window_level(if cfg.always_on_top {
                            WindowLevel::AlwaysOnTop
                        } else {
                            WindowLevel::Normal
                        });
                    }
                    if rebuild_show {
                        let (day, yday) = show.day_state();
                        let now = clock::now();
                        let mut profile_clock = now;
                        if cfg.custom_enabled && cfg.test_time != config::TestTime::Any {
                            profile_clock.hour = match (cfg.test_time, cfg.daynight) {
                                (config::TestTime::Any, _) => now.hour,
                                (config::TestTime::Day, DayNight::Original) => 1,
                                (config::TestTime::Night, DayNight::Original) => 0,
                                (config::TestTime::Day, DayNight::Real24h) => 12,
                                (config::TestTime::Night, DayNight::Real24h) => 23,
                            };
                        }
                        let director = Director::new(day, yday).with_daynight(cfg.daynight);
                        show = Show::new(&archive, &palette, 640, 480, director, profile_clock, seed);
                        show.set_holiday_animations(&archive, cfg.holiday_animations);
                        show.set_holiday_override(
                            &archive,
                            cfg.custom_enabled
                                .then(|| test_holiday_override(cfg.test_holiday))
                                .flatten(),
                        );
                        show.set_tide_preference(
                            &archive,
                            match if cfg.custom_enabled {
                                cfg.tide_preference
                            } else {
                                config::TidePreference::Authentic
                            } {
                                config::TidePreference::Authentic => TidePreference::Authentic,
                                config::TidePreference::PreferHigh => TidePreference::PreferHigh,
                                config::TidePreference::PreferLow => TidePreference::PreferLow,
                            },
                        );
                        enable_configured_test_playback(&mut show, &archive, &cfg);
                        show.set_island_offset_override(
                            &archive,
                            (cfg.custom_enabled && cfg.test_offset_enabled)
                                .then_some((cfg.test_offset_x, cfg.test_offset_y)),
                        );
                        if cfg.transition == config::Transition::Dissolve {
                            show.enable_dissolve();
                        }
                        next_wave_due = None;
                        pacer = pace::FramePacer::new();
                        last_rgba = None;
                    } else {
                        show.set_holiday_animations(&archive, cfg.holiday_animations);
                        show.set_tide_preference(
                            &archive,
                            match cfg.tide_preference {
                                config::TidePreference::Authentic => TidePreference::Authentic,
                                config::TidePreference::PreferHigh => TidePreference::PreferHigh,
                                config::TidePreference::PreferLow => TidePreference::PreferLow,
                            },
                        );
                    }
                    wave_period = Duration::from_millis(cfg.frame_delay_ms(WAVE_PERIOD_TICKS));
                    install_native_menu(&window, &cfg);
                    if usize::from(command) == MENU_PROFILE_CUSTOM && !was_custom_enabled {
                        // Selecting Custom Mode is the entry point to the tabbed
                        // controller.  The profile has already been saved and
                        // applied above; opening it here avoids making the user
                        // reopen Display just to find the controls.
                        open_custom_dialog(&mut custom_dialog);
                    }
                    if full_size_window_changed && runtime_windowed {
                        window.set_maximized(cfg.full_size_window);
                    }
                }
                if let Some(socket) = &preview_reload_socket {
                    // The editable 63-scene catalog travels with a Preview
                    // request.  Leave ample room for user-written descriptions
                    // instead of silently truncating them at the old 8 KiB limit.
                    let mut message = [0_u8; 65_507];
                    if let Ok((len, sender)) = socket.recv_from(&mut message) {
                        if let Ok(request) = std::str::from_utf8(&message[..len]) {
                            if request == "wilson-status" {
                                let (day, _) = show.day_state();
                                let scene = show
                                    .debug_info()
                                    .scene
                                    .and_then(|(name, tag)| {
                                        STORY_SCENES.iter().position(|entry| {
                                            entry.ads_name == name && entry.ads_tag == tag
                                        })
                                    })
                                    .map(|index| index + 1)
                                    .unwrap_or(0);
                                let reply = format!("day:{day};scene:{scene}");
                                let _ = socket.send_to(reply.as_bytes(), sender);
                                return;
                            }
                            if let Some(wire) = request.strip_prefix("wilson-preview-config:") {
                                let requested_cfg = config::Config::from_preview_wire(wire);
                                // Winit cannot change an existing window between framed and
                                // borderless fullscreen. Every other setting applies now.
                                let window_mode_pending = requested_cfg.windowed != cfg.windowed;
                                let current_windowed = runtime_windowed;
                                cfg = requested_cfg;
                                cfg.windowed = current_windowed;
                                audio.set_muted(audio_muted(cfg.mute, is_preview));
                                if runtime_windowed {
                                    window.set_window_level(if cfg.always_on_top {
                                        WindowLevel::AlwaysOnTop
                                    } else {
                                        WindowLevel::Normal
                                    });
                                }
                                let now = clock::now();
                                let mut preview_clock = now;
                                if cfg.custom_enabled && cfg.test_time != config::TestTime::Any {
                                    preview_clock.hour = match (cfg.test_time, cfg.daynight) {
                                        (config::TestTime::Any, _) => now.hour,
                                        (config::TestTime::Day, DayNight::Original) => 1,
                                        (config::TestTime::Night, DayNight::Original) => 0,
                                        (config::TestTime::Day, DayNight::Real24h) => 12,
                                        (config::TestTime::Night, DayNight::Real24h) => 23,
                                    };
                                }
                                let director = Director::new(1, now.yday).with_daynight(cfg.daynight);
                                show = Show::new(&archive, &palette, 640, 480, director, preview_clock, seed);
                                show.set_holiday_animations(&archive, cfg.holiday_animations);
                                show.set_holiday_override(
                                    &archive,
                                    cfg.custom_enabled
                                        .then(|| test_holiday_override(cfg.test_holiday))
                                        .flatten(),
                                );
                                show.set_tide_preference(
                                    &archive,
                                    match if cfg.custom_enabled {
                                        cfg.tide_preference
                                    } else {
                                        config::TidePreference::Authentic
                                    } {
                                        config::TidePreference::Authentic => TidePreference::Authentic,
                                        config::TidePreference::PreferHigh => TidePreference::PreferHigh,
                                        config::TidePreference::PreferLow => TidePreference::PreferLow,
                                    },
                                );
                                enable_configured_test_playback(&mut show, &archive, &cfg);
                                show.set_island_offset_override(
                                    &archive,
                                    (cfg.custom_enabled && cfg.test_offset_enabled)
                                        .then_some((cfg.test_offset_x, cfg.test_offset_y)),
                                );
                                // Preview resets the scene/runtime in place; it is not a
                                // fresh launch and must not replay the startup intro.
                                if cfg.transition == config::Transition::Dissolve {
                                    show.enable_dissolve();
                                }
                                wave_period = Duration::from_millis(cfg.frame_delay_ms(WAVE_PERIOD_TICKS));
                                next_wave_due = None;
                                pacer = pace::FramePacer::new();
                                last_rgba = None;
                                // A Custom apply can also switch the saved startup
                                // profile from Original to Custom. Update only those
                                // checkmarks; replacing the full native menu here can
                                // interrupt the live preview on Windows.
                                refresh_profile_menu_checks(&window, &cfg);
                                if window_mode_pending && cfg.debug {
                                    eprintln!("[wilson:debug] Preview queued window-mode change for next launch");
                                }
                                let _ = socket.send_to(b"ok", sender);
                                window.request_redraw();
                                return;
                            }
                            if let Some((
                                scene,
                                time,
                                offset_enabled,
                                offset_x,
                                offset_y,
                                tide_preference,
                            )) = request
                                .strip_prefix("wilson-preview:")
                                .and_then(parse_preview_options)
                            {
                                cfg.test_scene = scene;
                                cfg.test_time = time;
                                cfg.test_offset_enabled = offset_enabled;
                                cfg.test_offset_x = offset_x;
                                cfg.test_offset_y = offset_y;
                                cfg.tide_preference = tide_preference;
                                let now = clock::now();
                                let mut preview_clock = now;
                                preview_clock.hour = match (time, cfg.daynight) {
                                    (config::TestTime::Any, _) => now.hour,
                                    (config::TestTime::Day, DayNight::Original) => 1,
                                    (config::TestTime::Night, DayNight::Original) => 0,
                                    (config::TestTime::Day, DayNight::Real24h) => 12,
                                    (config::TestTime::Night, DayNight::Real24h) => 23,
                                };
                                let director =
                                    Director::new(1, now.yday).with_daynight(cfg.daynight);
                                show = Show::new(
                                    &archive,
                                    &palette,
                                    640,
                                    480,
                                    director,
                                    preview_clock,
                                    seed,
                                );
                                show.set_tide_preference(
                                    &archive,
                                    match tide_preference {
                                        config::TidePreference::Authentic => {
                                            TidePreference::Authentic
                                        }
                                        config::TidePreference::PreferHigh => {
                                            TidePreference::PreferHigh
                                        }
                                        config::TidePreference::PreferLow => {
                                            TidePreference::PreferLow
                                        }
                                    },
                                );
                                if scene != 0 {
                                    let offset = offset_enabled.then_some((offset_x, offset_y));
                                    show.enable_scene_test_repeat(
                                        &archive,
                                        usize::from(scene - 1),
                                        offset,
                                    );
                                }
                                show.set_island_offset_override(
                                    &archive,
                                    offset_enabled.then_some((offset_x, offset_y)),
                                );
                                pacer = pace::FramePacer::new();
                                last_rgba = None;
                                let _ = socket.send_to(b"ok", sender);
                                window.request_redraw();
                                return;
                            }
                        }
                        if &message[..len] == b"wilson-save-preview" {
                            // Backward-compatible shutdown request from an older
                            // settings panel. New panels restart in place above.
                            let _ = socket.send_to(b"ok", sender);
                            elwt.exit();
                            return;
                        }
                    }
                }
                if cfg.debug && matches!(cause, StartCause::Init) {
                    eprintln!("[wilson:debug] event loop started (NewEvents::Init)");
                }
                window.request_redraw();
            }
            Event::Resumed => {
                if cfg.debug {
                    eprintln!("[wilson:debug] resumed");
                }
            }
            Event::LoopExiting => {
                #[cfg(windows)]
                close_custom_dialog(&mut custom_dialog);
                #[cfg(windows)]
                close_custom_dialog(&mut island_location_bar);
                if cfg.debug {
                    eprintln!("[wilson:debug] loop exiting (process will end)");
                }
                stats.total_secs = base_secs + session_start.elapsed().as_secs();
                report_persistence_error(stats.save());
            }
            _ => {}
        })
        .expect("event loop error");
}

/// What the screensaver was asked to do (from the Windows `/s` `/p` `/c` verbs).
/// Open the normal EXE beside the SCR for a Windows Control Panel `/c` request.
/// The EXE retains every existing Display menu and gives the user a live preview,
/// so `/c` never becomes a second, incomplete settings implementation.
fn launch_sibling_configuration_app() -> Result<(), String> {
    let saver = std::env::current_exe()
        .map_err(|error| format!("Could not locate the running screen saver: {error}"))?;
    let app = sibling_configuration_exe(&saver)?;
    let status = std::process::Command::new(&app)
        .args(["--windowed", "--no-intro"])
        // `/c` is conventionally modal: retain the SCR configuration request
        // until the user closes the EXE. This also prevents Control Panel from
        // treating the child as an orphaned background process.
        .status()
        .map_err(|error| format!("Could not open {}: {error}", app.display()))?;
    if !status.success() {
        return Err(format!(
            "Configuration app {} exited with {status}.",
            app.display()
        ));
    }
    Ok(())
}

fn sibling_configuration_exe(saver: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let app = saver.with_extension("exe");
    if app == saver {
        return Err(
            "The Windows Settings button must start the .scr file, not the EXE.".to_owned(),
        );
    }
    if !app.is_file() {
        return Err(format!(
            "Configuration app not found beside the screen saver: {}",
            app.display()
        ));
    }
    Ok(app)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Run the ordinary EXE experience (default).
    Run,
    /// Show a true Windows screensaver session (`/s`).
    Show,
    /// Print/show the configuration (`/c`).
    Configure,
    /// Render the small preview embedded in the parent window handle (`/p <hwnd>`).
    Preview(isize),
}

/// Detect the screensaver action in `args`. Verbs may use `/` or `-`, be upper/lower
/// case, and carry an HWND either after a colon (`/p:1234`) or as the next argument
/// (`/p 1234`).
fn screensaver_action(args: &[String]) -> Action {
    let mut i = 1;
    while i < args.len() {
        let low = args[i].to_ascii_lowercase();
        if let Some(body) = low.strip_prefix('/').or_else(|| low.strip_prefix('-')) {
            let mut chars = body.chars();
            let verb = chars.next();
            let rest = chars.as_str(); // text after the verb letter
            let clean = rest.is_empty() || rest.starts_with(':');
            match verb {
                Some('c') if clean => return Action::Configure,
                Some('s') if clean => return Action::Show,
                Some('p') if clean => {
                    let hwnd = rest
                        .strip_prefix(':')
                        .and_then(parse_hwnd)
                        .or_else(|| args.get(i + 1).and_then(|n| parse_hwnd(n)))
                        .unwrap_or(0);
                    return Action::Preview(hwnd);
                }
                _ => {}
            }
        }
        i += 1;
    }
    Action::Run
}

/// Parse a window handle (decimal, or `0x`-prefixed hex), as Windows passes it.
fn parse_hwnd(s: &str) -> Option<isize> {
    let s = s.trim();
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => isize::from_str_radix(hex, 16).ok(),
        None => s.parse::<isize>().ok(),
    }
}

/// Whether audio should be silenced: when muted by config, **or** in the Windows preview pane
/// (the little monitor in the Screen Saver settings must never play sound).
fn audio_muted(cfg_mute: bool, is_preview: bool) -> bool {
    cfg_mute || is_preview
}

/// Apply the Windows preview-window settings: make the window a borderless child of the
/// preview pane (`hwnd`). A no-op on other platforms (where `/p` isn't used).
fn apply_preview(builder: WindowBuilder, hwnd: isize) -> WindowBuilder {
    #[cfg(windows)]
    {
        use std::num::NonZeroIsize;
        use winit::raw_window_handle::{RawWindowHandle, Win32WindowHandle};
        // Fill the host preview pane: query its actual client size instead of guessing, so the
        // screensaver covers the whole little monitor (no black bands on the right/bottom). Fall
        // back to the classic ~152×112 if the query fails.
        let (w, h) = preview_pane_size(hwnd).unwrap_or((152, 112));
        let b = builder
            .with_decorations(false)
            .with_inner_size(winit::dpi::PhysicalSize::new(w, h));
        if let Some(nz) = NonZeroIsize::new(hwnd) {
            let handle = RawWindowHandle::Win32(Win32WindowHandle::new(nz));
            // SAFETY: `hwnd` is the preview window handle Windows passed on the command
            // line; it is valid for the lifetime of the preview.
            return unsafe { b.with_parent_window(Some(handle)) };
        }
        b
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        builder
    }
}

/// Query the client size (in px) of the Windows preview pane `hwnd`, so the embedded child can
/// fill it exactly. `None` if the handle is null or the query fails.
#[cfg(windows)]
fn preview_pane_size(hwnd: isize) -> Option<(u32, u32)> {
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetClientRect(hwnd: isize, rect: *mut Rect) -> i32;
    }
    if hwnd == 0 {
        return None;
    }
    let mut r = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `hwnd` is the preview window handle Windows passed; `r` is a valid out-param.
    let ok = unsafe { GetClientRect(hwnd, &mut r) };
    if ok == 0 {
        return None;
    }
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    (w > 0 && h > 0).then_some((w as u32, h as u32))
}

/// Whether `args` ask for help, accepting both Unix (`-h`, `--help`) and Windows
/// (`/?`, `/help`) conventions (case-insensitive).
fn wants_help(args: &[String]) -> bool {
    args.iter().skip(1).any(|a| {
        matches!(
            a.to_ascii_lowercase().as_str(),
            "-h" | "-help" | "--help" | "help" | "/?" | "/h" | "/help"
        )
    })
}

/// Print detailed, platform-appropriate usage. Unix shows `-h/--help`; Windows also
/// lists the screensaver verbs (`/s /c /p /?`). The `--…` options work on every platform.
fn print_help() {
    let help_flags = if cfg!(windows) {
        "/?, /help"
    } else {
        "-h, --help"
    };
    let cfg_path = config::config_file()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown location)".to_string());

    println!(
        "Castaway Continuum — a modern, portable clone of the 1992 \"Johnny Castaway\" \
         screensaver (Sierra/Dynamix).\n"
    );
    println!("USAGE:");
    println!("  wilson [OPTIONS]                  run the screensaver (fullscreen)");
    println!("  wilson --data <DIR> [OPTIONS]     use the original game data in <DIR>\n");
    println!("DATA (required):");
    println!("  Needs the original files RESOURCE.MAP + RESOURCE.001 (and soundN.wav for");
    println!(
        "  sound). Located via --data <DIR>, the CASTAWAY_CONTINUUM_DATA_DIR env var, the current"
    );
    println!("  directory, or next to the executable; file names match case-insensitively.\n");
    println!("OPTIONS:");
    println!("  {help_flags:<33}show this help and exit");
    println!("  --windowed                       run in a 640x480 window (default: fullscreen)");
    println!("  --mute                           disable sound effects (default: sound on)");
    println!("  --speed <PCT>                    playback speed 25-400; 100 = original (default)");
    println!("  --scale <MODE>                   fit | stretch | integer | extend (default: fit)");
    println!("                                     extend = fill widescreen, no bars/distortion");
    println!("  --filter <nearest|linear|xbr|xbrz> pixel sampling (default: linear):");
    println!("                                     nearest = crisp/retro,");
    println!("                                     linear  = smooth (bilinear, default),");
    println!(
        "                                     xbr     = smooth + sharp, dissolves dither (\"HD\"),"
    );
    println!("                                     xbrz    = smooth edges, keeps dither texture");
    println!("  --dedither                       smooth the dithered sea/sky (default: off)");
    println!("  --debug                          diagnostics: stdout status + on-screen HUD");
    println!("  --daynight <original|real24h>    day/night cycle (default: original 8h)");
    println!("  --no-intro                       skip the startup intro screen (default: shown)");
    println!(
        "  --intro-secs <S>                 how long to hold the intro screen (1-30; default 3)"
    );
    println!("  --day <1-11>                     start the 11-day story arc at this day");
    println!("  --story                          play the whole arc in order (day 1->11->1...)");
    println!("  --story-secs <S>                 story mode: real seconds per day (default 90)");
    println!(
        "  --special-cycle                  diagnostic: cycle the 11 day-specific specials only"
    );
    println!(
        "  --test-group <GROUP>             diagnostic: off | normal | special | visitors | common | uncommon | all"
    );
    println!(
        "  --special-days <1,3,5>           in special mode, cycle only the selected story days"
    );
    println!("  --test-time <TIME>               diagnostic lighting: any | day | night");
    println!("  --transition <none|dissolve>     scene transition (default: none = hard cut)");
    println!("  --data <DIR>                     game data folder (default: auto-detect)");
    if cfg!(windows) {
        println!("\nWINDOWS SCREENSAVER VERBS (as the OS invokes a .scr):");
        println!("  /s                               show the screensaver (same as no args)");
        println!("  /c                               configure (opens the settings file)");
        println!("  /p <HWND>                        preview inside the configuration pane");
        println!("  /?, /help                        show this help");
    }
    println!("\nCONFIG FILE (persists options; flags override it for one run):");
    println!("  {cfg_path}");
    println!("\nEXAMPLES:");
    if cfg!(windows) {
        println!("  CastawayContinuum.scr /c         show the configuration");
        println!("  CastawayContinuum.exe --windowed --speed 200");
    } else {
        println!("  wilson --data ~/jc --filter xbr");
        println!("  wilson --windowed --speed 200 --scale integer");
    }
    println!("\nFree software under GPL-3.0-or-later. Plays only your own original game data.");
}

/// Warp the actual picture area onto a gently rounded CRT face. Letterbox bars
/// deliberately remain untouched: they are outside the virtual display tube.
///
/// This is an original, compact CPU implementation rather than an imported
/// RetroArch shader. It uses inverse barrel mapping so every destination pixel
/// chooses a source sample, preventing the gaps that forward pixel pushing
/// would leave behind.
fn apply_crt_curvature(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    amount: config::CrtCurvature,
    content_rect: (usize, usize, usize, usize),
) {
    let (curve, vignette) = match amount {
        config::CrtCurvature::Off => return,
        config::CrtCurvature::Gentle => (0.025_f32, 0.12_f32),
        config::CrtCurvature::Curved => (0.050_f32, 0.20_f32),
        config::CrtCurvature::Strong => (0.075_f32, 0.28_f32),
    };
    let (x0, y0, width, height) = content_rect;
    if width < 4 || height < 4 || x0 >= dw || y0 >= dh || x0 + width > dw || y0 + height > dh {
        return;
    }

    // Copy only the real picture, not the entire window (which may include
    // black Fit/Integer bars). This also makes a normal 640x480 window cheap.
    let mut source = Vec::with_capacity(width * height);
    for y in y0..y0 + height {
        source.extend_from_slice(&buf[y * dw + x0..y * dw + x0 + width]);
    }

    let max_x = (width - 1) as f32;
    let max_y = (height - 1) as f32;
    // Values are in the -1..1 tube coordinate space. A CRT's glass edge is
    // crisp, so use just enough softening for antialiasing rather than a blur.
    const CORNER_RADIUS: f32 = 0.080;
    let edge_softness = 2.5 / width.min(height) as f32;

    for local_y in 0..height {
        let ny = (local_y as f32 / max_y) * 2.0 - 1.0;
        for local_x in 0..width {
            let nx = (local_x as f32 / max_x) * 2.0 - 1.0;
            // Edges bend away from the viewer. Sampling outward gives the
            // characteristic rounded, clipped corners of a curved tube.
            let sx = nx * (1.0 + curve * ny * ny);
            let sy = ny * (1.0 + curve * nx * nx);
            let target = (y0 + local_y) * dw + x0 + local_x;
            // Feather the warped screen edge instead of abruptly switching to
            // black. Sample the clamped edge pixel under that feather so the
            // display fades naturally into its black surround.
            let tube_alpha = ((1.0 - sx.abs()).min(1.0 - sy.abs()) / edge_softness).clamp(0.0, 1.0);
            if tube_alpha <= 0.0 {
                buf[target] = 0;
                continue;
            }

            // Bilinear resampling stops the warp from creating little stair
            // steps, while retaining the selected pixel filter's character.
            let sample_x = ((sx.clamp(-1.0, 1.0) + 1.0) * 0.5) * max_x;
            let sample_y = ((sy.clamp(-1.0, 1.0) + 1.0) * 0.5) * max_y;
            let ix = sample_x.floor() as usize;
            let iy = sample_y.floor() as usize;
            let fx = sample_x - ix as f32;
            let fy = sample_y - iy as f32;
            let ix1 = (ix + 1).min(width - 1);
            let iy1 = (iy + 1).min(height - 1);
            let color = bilerp_argb(
                source[iy * width + ix],
                source[iy * width + ix1],
                source[iy1 * width + ix],
                source[iy1 * width + ix1],
                fx,
                fy,
            );

            // A CRT face has rounded corners and gets subtly darker toward
            // the glass edge. Keep this restrained: it should feel like a TV,
            // not a black-hole vignette pasted over Johnny.
            let corner_x = (sx.abs() - (1.0 - CORNER_RADIUS)).max(0.0);
            let corner_y = (sy.abs() - (1.0 - CORNER_RADIUS)).max(0.0);
            let corner_distance = (corner_x * corner_x + corner_y * corner_y).sqrt();
            if corner_distance >= CORNER_RADIUS {
                buf[target] = 0;
                continue;
            }
            let corner_alpha = ((CORNER_RADIUS - corner_distance) / edge_softness).clamp(0.0, 1.0);
            let edge = sx.abs().max(sy.abs());
            let edge_amount = ((edge - 0.72) / 0.28).clamp(0.0, 1.0);
            let brightness =
                (1.0 - vignette * edge_amount * edge_amount) * corner_alpha * tube_alpha;
            buf[target] = dim_argb(color, brightness);
        }
    }
}

fn bilerp_argb(
    top_left: u32,
    top_right: u32,
    bottom_left: u32,
    bottom_right: u32,
    fx: f32,
    fy: f32,
) -> u32 {
    let channel = |shift: u32| {
        let tl = ((top_left >> shift) & 0xFF) as f32;
        let tr = ((top_right >> shift) & 0xFF) as f32;
        let bl = ((bottom_left >> shift) & 0xFF) as f32;
        let br = ((bottom_right >> shift) & 0xFF) as f32;
        let top = tl + (tr - tl) * fx;
        let bottom = bl + (br - bl) * fx;
        (top + (bottom - top) * fy).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn dim_argb(color: u32, brightness: f32) -> u32 {
    let scale = (brightness * 255.0).round() as u32;
    let r = ((color >> 16) & 0xFF) * scale / 255;
    let g = ((color >> 8) & 0xFF) * scale / 255;
    let b = (color & 0xFF) * scale / 255;
    (r << 16) | (g << 8) | b
}

/// Adjust the finished picture's response without touching black letterbox
/// bars or the operating-system chrome. The profiles are deliberately mild:
/// they should suggest a tube, not repaint Sierra's artwork.
fn apply_crt_color_profile(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    profile: config::CrtColorProfile,
    content_rect: (usize, usize, usize, usize),
) {
    if profile == config::CrtColorProfile::Neutral {
        return;
    }
    let (x0, y0, width, height) = content_rect;
    if width == 0 || height == 0 || x0 >= dw || y0 >= dh || x0 + width > dw || y0 + height > dh {
        return;
    }
    let clamp = |value: i32| value.clamp(0, 255) as u32;
    for y in y0..y0 + height {
        for x in x0..x0 + width {
            let pixel = buf[y * dw + x];
            if pixel == 0 {
                continue;
            }
            let mut r = ((pixel >> 16) & 0xFF) as i32;
            let mut g = ((pixel >> 8) & 0xFF) as i32;
            let mut b = (pixel & 0xFF) as i32;
            let luma = (r * 77 + g * 150 + b * 29) >> 8;
            match profile {
                config::CrtColorProfile::Neutral => unreachable!(),
                config::CrtColorProfile::Classic => {
                    // Mild saturation plus a barely perceptible contrast lift.
                    r = luma + (r - luma) * 106 / 100;
                    g = luma + (g - luma) * 106 / 100;
                    b = luma + (b - luma) * 106 / 100;
                    r = (r - 128) * 104 / 100 + 128;
                    g = (g - 128) * 104 / 100 + 128;
                    b = (b - 128) * 104 / 100 + 128;
                }
                config::CrtColorProfile::Warm => {
                    // Consumer TV warmth: a gentle red lift and blue reduction,
                    // balanced so daylight skies do not turn into gravy.
                    r = (r - 128) * 102 / 100 + 128;
                    g = (g - 128) * 101 / 100 + 128;
                    b = (b - 128) * 101 / 100 + 128;
                    r = r * 105 / 100;
                    b = b * 92 / 100;
                }
            }
            buf[y * dw + x] = (clamp(r) << 16) | (clamp(g) << 8) | clamp(b);
        }
    }
}

/// Blend pixels only along each scanline. This mimics a broader horizontal
/// electron beam while preserving vertical detail and the scene's scanline
/// structure.
fn apply_crt_horizontal_softness(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    strength: config::CrtHorizontalSoftness,
    content_rect: (usize, usize, usize, usize),
) {
    let (center_weight, side_weight) = match strength {
        config::CrtHorizontalSoftness::Off => return,
        config::CrtHorizontalSoftness::Light => (6u32, 1u32),
        config::CrtHorizontalSoftness::Classic => (4u32, 2u32),
    };
    let (x0, y0, width, height) = content_rect;
    if width < 3 || height == 0 || x0 >= dw || y0 >= dh || x0 + width > dw || y0 + height > dh {
        return;
    }
    let mut source = Vec::with_capacity(width * height);
    for y in y0..y0 + height {
        source.extend_from_slice(&buf[y * dw + x0..y * dw + x0 + width]);
    }
    let total = center_weight + side_weight * 2;
    for local_y in 0..height {
        for local_x in 1..width - 1 {
            let center = source[local_y * width + local_x];
            // Respect the curved tube's black boundary rather than smearing a
            // bright scene pixel into it.
            if center == 0 {
                continue;
            }
            let left = source[local_y * width + local_x - 1];
            let right = source[local_y * width + local_x + 1];
            let blend_channel = |shift: u32| {
                (((center >> shift) & 0xFF) * center_weight
                    + ((left >> shift) & 0xFF) * side_weight
                    + ((right >> shift) & 0xFF) * side_weight)
                    / total
            };
            buf[(y0 + local_y) * dw + x0 + local_x] =
                (blend_channel(16) << 16) | (blend_channel(8) << 8) | blend_channel(0);
        }
    }
}

/// Apply a small, bright-only glow from neighbouring pixels. This is not a
/// general blur: only highlights contribute, recreating the slight halation
/// that bright phosphors produce behind CRT glass.
fn apply_crt_bloom(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    strength: config::CrtBloom,
    color_profile: config::CrtColorProfile,
    horizontal_softness: config::CrtHorizontalSoftness,
    content_rect: (usize, usize, usize, usize),
) {
    apply_crt_color_profile(buf, dw, dh, color_profile, content_rect);
    apply_crt_horizontal_softness(buf, dw, dh, horizontal_softness, content_rect);
    if strength == config::CrtBloom::Off {
        return;
    }
    let (x0, y0, width, height) = content_rect;
    if width < 3 || height < 3 || x0 >= dw || y0 >= dh || x0 + width > dw || y0 + height > dh {
        return;
    }
    let mut source = Vec::with_capacity(width * height);
    for y in y0..y0 + height {
        source.extend_from_slice(&buf[y * dw + x0..y * dw + x0 + width]);
    }
    // Cardinal neighbours carry more of the glow than diagonals. The divisor
    // is deliberately large so this remains a faint halo, not a blurry filter.
    const NEIGHBOURS: [(isize, isize, u32); 8] = [
        (-1, 0, 20),
        (1, 0, 20),
        (0, -1, 20),
        (0, 1, 20),
        (-1, -1, 10),
        (1, -1, 10),
        (-1, 1, 10),
        (1, 1, 10),
    ];
    for local_y in 1..height - 1 {
        for local_x in 1..width - 1 {
            let source_pixel = source[local_y * width + local_x];
            // Curvature creates genuine black outside the glass. Do not let
            // bright pixels bleed a haze back into that crisp boundary.
            if source_pixel == 0 {
                continue;
            }
            let (mut add_r, mut add_g, mut add_b) = (0u32, 0u32, 0u32);
            for (dx, dy, weight) in NEIGHBOURS {
                let neighbour = source
                    [(local_y as isize + dy) as usize * width + (local_x as isize + dx) as usize];
                let r = (neighbour >> 16) & 0xFF;
                let g = (neighbour >> 8) & 0xFF;
                let b = neighbour & 0xFF;
                let luminance = (r * 77 + g * 150 + b * 29) >> 8;
                let excess = luminance.saturating_sub(185);
                add_r += r * excess * weight / (255 * 500);
                add_g += g * excess * weight / (255 * 500);
                add_b += b * excess * weight / (255 * 500);
            }
            let r = ((source_pixel >> 16) & 0xFF).saturating_add(add_r).min(255);
            let g = ((source_pixel >> 8) & 0xFF).saturating_add(add_g).min(255);
            let b = (source_pixel & 0xFF).saturating_add(add_b).min(255);
            buf[(y0 + local_y) * dw + x0 + local_x] = (r << 16) | (g << 8) | b;
        }
    }
}

/// Overlay a restrained RGB aperture-grille pattern over only the actual
/// picture. It is intentionally placed after curvature so black corners stay
/// black, and before scanlines so the two effects layer like a physical tube.
fn apply_crt_phosphor_mask(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    strength: config::CrtPhosphorMask,
    content_rect: (usize, usize, usize, usize),
) {
    let muted_channels = match strength {
        config::CrtPhosphorMask::Off => return,
        // Keep the light setting deliberately understated. At native 640x480
        // it should reward a close look rather than turn the scene into RGB
        // confetti.
        config::CrtPhosphorMask::Subtle => 242u32,
        config::CrtPhosphorMask::Visible => 218u32,
    };
    let (x0, y0, width, height) = content_rect;
    if width == 0 || height == 0 || x0 >= dw || y0 >= dh || x0 + width > dw || y0 + height > dh {
        return;
    }
    for y in y0..y0 + height {
        for x in x0..x0 + width {
            let pixel = buf[y * dw + x];
            let mut r = (pixel >> 16) & 0xFF;
            let mut g = (pixel >> 8) & 0xFF;
            let mut b = pixel & 0xFF;
            match (x - x0) % 3 {
                0 => {
                    g = g * muted_channels / 255;
                    b = b * muted_channels / 255;
                }
                1 => {
                    r = r * muted_channels / 255;
                    b = b * muted_channels / 255;
                }
                _ => {
                    r = r * muted_channels / 255;
                    g = g * muted_channels / 255;
                }
            }
            buf[y * dw + x] = (r << 16) | (g << 8) | b;
        }
    }
}

/// Apply a simple, crisp CRT scanline mask after scene/UI composition. The
/// scanlines therefore affect captions too, just as a physical CRT would.
fn apply_crt_scanlines(buf: &mut [u32], dw: usize, dh: usize, strength: config::CtrScanlines) {
    let keep = match strength {
        config::CtrScanlines::Off => return,
        config::CtrScanlines::Light => 220u32,
        config::CtrScanlines::Medium => 180u32,
        config::CtrScanlines::Strong => 135u32,
    };
    for y in (1..dh).step_by(2) {
        let row = &mut buf[y * dw..(y + 1) * dw];
        for pixel in row {
            let r = ((*pixel >> 16) & 0xFF) * keep / 255;
            let g = ((*pixel >> 8) & 0xFF) * keep / 255;
            let b = (*pixel & 0xFF) * keep / 255;
            *pixel = (r << 16) | (g << 8) | b;
        }
    }
}

/// Wrap text to the small built-in bitmap font's fixed-width display area.
fn scene_info_lines(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let needed = usize::from(!line.is_empty()) + word.len();
        if !line.is_empty() && line.len() + needed > max_chars {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Draw the friendly scene name and description in the selected corner of
/// the animation. The panel is blended with the picture rather than being an
/// opaque dialog; the scene stays visible behind it.
fn draw_scene_info_overlay(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    info: &wilson_engine::DebugInfo,
    cfg: &config::Config,
    corner: config::SceneInfoCorner,
    show_long_description: bool,
    curvature: config::CrtCurvature,
    content_rect: (usize, usize, usize, usize),
) {
    let Some((ads_name, ads_tag)) = info.scene else {
        return;
    };
    let Some((number, name, description)) =
        settings_ui::scene_catalog_text_for_program(cfg, ads_name, ads_tag)
    else {
        return;
    };
    // 640x480 is the compact window baseline. Keep the pleasant small font at
    // or below that size; scale it up only when the display genuinely has room.
    let (content_x, content_y, content_w, content_h) = content_rect;
    if content_w == 0 || content_h == 0 {
        return;
    }
    let (scale_numerator, scale_denominator): (usize, usize) =
        if content_w > 1_920 && content_h > 1_080 {
            (2, 1)
        } else if content_w > 640 && content_h > 480 {
            (3, 2)
        } else {
            (1, 1)
        };
    // The CRT pass is applied after this panel is painted. Reserve its curved
    // edge/corner area now, so every corner selection keeps the whole panel
    // and its text safely on the visible glass.
    let edge_inset = match curvature {
        config::CrtCurvature::Off => 0,
        config::CrtCurvature::Gentle => 0,
        config::CrtCurvature::Curved => 6,
        config::CrtCurvature::Strong => 12,
    };
    let pad = 6usize;
    let corner_inset = edge_inset
        .min(content_w.saturating_div(5))
        .min(content_h.saturating_div(5));
    let char_width = (6 * scale_numerator).div_ceil(scale_denominator);
    let max_chars =
        ((content_w.saturating_sub((pad + corner_inset) * 2)) / char_width).clamp(22, 50);
    let mut lines = vec![format!("SCENE {number:02} - {name}")];
    if show_long_description {
        lines.extend(scene_info_lines(description, max_chars));
    }
    let lh = font::line_height_ratio(scale_numerator, scale_denominator) + 3;
    let width = lines
        .iter()
        .map(|line| font::text_width_ratio(line, scale_numerator, scale_denominator))
        .max()
        .unwrap_or(0)
        .min(content_w.saturating_sub((pad + corner_inset) * 2));
    let height = lines.len() * lh + pad * 2;
    let panel_width = width + pad * 2;
    let (x, y) = match corner {
        config::SceneInfoCorner::TopLeft => (
            content_x + pad + corner_inset,
            content_y + pad + corner_inset,
        ),
        config::SceneInfoCorner::TopRight => (
            content_x + content_w.saturating_sub(panel_width + pad + corner_inset),
            content_y + pad + corner_inset,
        ),
        config::SceneInfoCorner::BottomLeft => (
            content_x + pad + corner_inset,
            content_y + content_h.saturating_sub(height + pad + corner_inset),
        ),
        config::SceneInfoCorner::BottomRight => (
            content_x + content_w.saturating_sub(panel_width + pad + corner_inset),
            content_y + content_h.saturating_sub(height + pad + corner_inset),
        ),
    };

    // 70% dark navy, 30% underlying scene: readable without pretending the
    // animation beneath the label vanished.
    for py in y..(y + height).min(dh) {
        for px in x..(x + panel_width).min(dw) {
            let pixel = buf[py * dw + px];
            let r = ((pixel >> 16) & 0xFF) * 3 / 10;
            let g = ((pixel >> 8) & 0xFF) * 3 / 10;
            let b = (pixel & 0xFF) * 4 / 10;
            buf[py * dw + px] = (r << 16) | (g << 8) | b;
        }
    }
    for (i, line) in lines.iter().enumerate() {
        let color = if i == 0 { 0x00FF_FFE0 } else { 0x00FF_FFFF };
        font::draw_text_ratio(
            buf,
            dw,
            dh,
            x + pad,
            y + pad + i * lh,
            line,
            scale_numerator,
            scale_denominator,
            color,
        );
    }
}

/// Draw the `--debug` HUD (top-left) into the window buffer: a dark panel with the live
/// runtime state, so a single screenshot captures everything needed to diagnose a report.
fn draw_debug_hud(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    fps: u32,
    delay_ticks: u16,
    info: &wilson_engine::DebugInfo,
    cfg: &config::Config,
) {
    let scene = info
        .scene
        .map(|(n, t)| format!("{n}#{t}"))
        .unwrap_or_else(|| "-".into());
    let off = info
        .offset
        .map(|(x, y)| format!("{x},{y}"))
        .unwrap_or_else(|| "-".into());
    let lines = [
        "WILSON DEBUG".to_string(),
        format!("FPS {fps}  FRAME {delay_ticks}T"),
        format!("DAY {}/11  STAGE {}", info.day, info.stage.to_uppercase()),
        format!("SCENE {scene}"),
        format!("DRIFT {off}  ISLAND {}", info.on_island as u8),
        format!(
            "NIGHT {} TIDE {} RAFT {}",
            info.night as u8, info.low_tide as u8, info.raft
        ),
        format!(
            "FILTER {}  SCALE {}",
            cfg.filter.as_str().to_uppercase(),
            cfg.scale.as_str().to_uppercase()
        ),
    ];
    let scale = 2usize;
    let pad = 4usize;
    let lh = font::line_height(scale) + 2;
    let w = lines
        .iter()
        .map(|l| font::text_width(l, scale))
        .max()
        .unwrap_or(0)
        + pad * 2;
    let h = lines.len() * lh + pad * 2;
    font::fill_rect(buf, dw, dh, 0, 0, w, h, 0x0000_0000); // black panel
    for (i, line) in lines.iter().enumerate() {
        font::draw_text(buf, dw, dh, pad, pad + i * lh, line, scale, 0x0000_FF00);
        // green
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("wilson")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn modifier_and_synthetic_keys_do_not_dismiss() {
        // The bug: a fullscreen window on Windows gets a spurious AltGraph press on focus,
        // closing the screensaver instantly. Modifier keys (and synthetic focus events)
        // must NOT dismiss.
        for m in [
            NamedKey::AltGraph,
            NamedKey::Alt,
            NamedKey::Control,
            NamedKey::Shift,
            NamedKey::Super,
            NamedKey::CapsLock,
            NamedKey::NumLock,
        ] {
            assert!(
                !key_dismisses(&Key::Named(m), ElementState::Pressed, false),
                "modifier {m:?} must not dismiss"
            );
        }
        // Synthetic events (delivered for keys held when focus changes) never dismiss.
        assert!(!key_dismisses(
            &Key::Named(NamedKey::Escape),
            ElementState::Pressed,
            true
        ));
        // Key releases never dismiss.
        assert!(!key_dismisses(
            &Key::Named(NamedKey::Space),
            ElementState::Released,
            false
        ));
    }

    #[test]
    fn real_key_presses_dismiss() {
        for k in [
            NamedKey::Escape,
            NamedKey::Space,
            NamedKey::Enter,
            NamedKey::ArrowLeft,
        ] {
            assert!(
                key_dismisses(&Key::Named(k), ElementState::Pressed, false),
                "real key {k:?} should dismiss"
            );
        }
        // A character key dismisses too.
        assert!(key_dismisses(
            &Key::Character("a".into()),
            ElementState::Pressed,
            false
        ));
    }

    #[test]
    fn actions_are_detected() {
        assert_eq!(screensaver_action(&args(&["/c"])), Action::Configure);
        assert_eq!(screensaver_action(&args(&["-s"])), Action::Show);
        assert_eq!(screensaver_action(&args(&["/S:0"])), Action::Show);
        assert_eq!(screensaver_action(&args(&[])), Action::Run);
    }

    #[test]
    fn control_panel_configuration_uses_the_sibling_exe() {
        let root =
            std::env::temp_dir().join(format!("wilson-config-launcher-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let saver = root.join("JohnnyCastaway.scr");
        let app = root.join("JohnnyCastaway.exe");
        std::fs::write(&saver, []).unwrap();
        std::fs::write(&app, []).unwrap();
        assert_eq!(sibling_configuration_exe(&saver), Ok(app));
        assert!(sibling_configuration_exe(&root.join("missing.scr")).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn help_is_detected_per_platform() {
        // Unix and Windows conventions, case-insensitive.
        for f in [
            "-h", "--help", "-help", "help", "/?", "/h", "/help", "--HELP", "/Help",
        ] {
            assert!(wants_help(&args(&[f])), "should detect help flag {f:?}");
        }
        // Real options must not be mistaken for help.
        assert!(!wants_help(&args(&[])));
        assert!(!wants_help(&args(&["--windowed", "--filter", "xbr"])));
        assert!(!wants_help(&args(&["/c"])));
        assert!(!wants_help(&args(&["--data", "/some/dir"])));
    }

    #[test]
    fn preview_action_captures_hwnd() {
        // HWND as the next argument, or after a colon, or absent (0).
        assert_eq!(
            screensaver_action(&args(&["/p", "1234"])),
            Action::Preview(1234)
        );
        assert_eq!(
            screensaver_action(&args(&["/P:5678"])),
            Action::Preview(5678)
        );
        assert_eq!(screensaver_action(&args(&["-p"])), Action::Preview(0));
    }

    #[test]
    fn preview_pane_is_always_silent() {
        // Regression: the little preview monitor in the Screen Saver settings ran the engine
        // with sound. It must be muted regardless of the config's mute setting.
        assert!(
            audio_muted(false, true),
            "preview must be silent even when not muted"
        );
        assert!(
            audio_muted(true, true),
            "preview stays silent when muted too"
        );
        assert!(
            audio_muted(true, false),
            "config mute still mutes a normal run"
        );
        assert!(!audio_muted(false, false), "a normal run plays sound");
    }

    #[test]
    fn non_verbs_are_ignored() {
        assert_eq!(
            screensaver_action(&args(&["--data", "/some/dir"])),
            Action::Run
        );
        assert_eq!(
            screensaver_action(&args(&["--windowed", "--scale", "fit"])),
            Action::Run
        );
    }

    #[test]
    fn parse_hwnd_decimal_and_hex() {
        assert_eq!(parse_hwnd("1234"), Some(1234));
        assert_eq!(parse_hwnd("0x10"), Some(16));
        assert_eq!(parse_hwnd(" 42 "), Some(42));
        assert_eq!(parse_hwnd("nope"), None);
    }

    #[cfg(windows)]
    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn custom_menu_commands_preserve_and_gate_profile_values() {
        let mut cfg = config::Config::default();
        cfg.test_offset_enabled = true;
        cfg.test_offset_x = 25;
        cfg.test_offset_y = -15;

        // Classic disables Custom controls but retains the user's saved values.
        assert!(apply_native_menu_command(
            &mut cfg,
            MENU_PROFILE_ORIGINAL as u16
        ));
        assert!(!cfg.custom_enabled);
        assert!(!apply_native_menu_command(
            &mut cfg,
            MENU_LIGHTING_NIGHT as u16
        ));
        assert_eq!(cfg.test_time, config::TestTime::Any);
        assert!(cfg.test_offset_enabled);
        assert_eq!((cfg.test_offset_x, cfg.test_offset_y), (25, -15));

        // Re-enabling Custom makes the stored profile usable again, and an
        // environment command must keep Custom active for future launches.
        assert!(apply_native_menu_command(
            &mut cfg,
            MENU_PROFILE_CUSTOM as u16
        ));
        assert!(apply_native_menu_command(
            &mut cfg,
            MENU_LIGHTING_NIGHT as u16
        ));
        assert!(cfg.custom_enabled);
        assert_eq!(cfg.test_time, config::TestTime::Night);
        assert!(apply_native_menu_command(&mut cfg, MENU_TIDE_LOW as u16));
        assert_eq!(cfg.tide_preference, config::TidePreference::PreferLow);
    }
}
