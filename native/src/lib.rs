#![cfg(windows)]

mod clipboard;
mod system;

pub use system::{
    battery_charging, battery_full_seconds, battery_on_ac_power, battery_percent,
    battery_remaining_seconds, battery_saver, cpu_count, disable_power_saving, enable_power_saving,
    hostname, lock, restart, shutdown, sleep, uptime, username,
};

pub use clipboard::Clipboard;

mod audio;

pub use audio::Volume;