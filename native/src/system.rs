use windows::core::{Result, GUID, PWSTR};

use windows::Win32::System::{
    Power::{GetSystemPowerStatus, PowerSetActiveScheme, SetSuspendState, SYSTEM_POWER_STATUS},
    Shutdown::{
        ExitWindowsEx, LockWorkStation, EWX_REBOOT, EWX_SHUTDOWN, SHTDN_REASON_FLAG_PLANNED,
        SHTDN_REASON_MAJOR_APPLICATION,
    },
    SystemInformation::{GetSystemInfo, GetTickCount64},
    SystemServices::GUID_MIN_POWER_SAVINGS,
    WindowsProgramming::{GetComputerNameW, GetUserNameW},
};

const GUID_BALANCED: GUID = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);

//
// ─────────────────────────────────────────────────────────────
// System Information
// ─────────────────────────────────────────────────────────────
//

/// Returns the Windows computer hostname.
pub fn hostname() -> String {
    hostname_inner().unwrap_or_default()
}

/// Retrieves the Windows computer hostname internally.
fn hostname_inner() -> Result<String> {
    let mut buffer = vec![0u16; 256];
    let mut size = buffer.len() as u32;

    unsafe {
        GetComputerNameW(Some(PWSTR(buffer.as_mut_ptr())), &mut size)?;
    }

    Ok(String::from_utf16_lossy(&buffer[..size as usize]))
}

/// Returns the username of the currently logged-in Windows user.
pub fn username() -> String {
    username_inner().unwrap_or_default()
}

/// Retrieves the Windows username internally.
fn username_inner() -> Result<String> {
    let mut buffer = vec![0u16; 256];
    let mut size = buffer.len() as u32;

    unsafe {
        GetUserNameW(Some(PWSTR(buffer.as_mut_ptr())), &mut size)?;
    }

    let len = if size > 0 && buffer[(size - 1) as usize] == 0 {
        size - 1
    } else {
        size
    };

    Ok(String::from_utf16_lossy(&buffer[..len as usize]))
}

/// Returns the number of logical processors available to Windows.
pub fn cpu_count() -> u32 {
    let mut info = Default::default();

    unsafe {
        GetSystemInfo(&mut info);
    }

    info.dwNumberOfProcessors
}

/// Returns the amount of time Windows has been running in milliseconds.
pub fn uptime() -> u64 {
    unsafe { GetTickCount64() }
}

//
// ─────────────────────────────────────────────────────────────
// Session / Machine Control
// ─────────────────────────────────────────────────────────────
//

/// Locks the current Windows session.
///
/// Returns `true` when the lock request was successfully submitted.
pub fn lock() -> bool {
    unsafe { LockWorkStation().is_ok() }
}

/// Puts Windows into sleep mode.
///
/// Returns `true` when Windows accepts the sleep request.
pub fn sleep() -> bool {
    unsafe {
        SetSuspendState(
            false, // hibernate
            false, // force
            false, // disable wake events
        )
    }
}

/// Shuts down Windows using the normal shutdown sequence.
///
/// Returns `true` when the shutdown request was successfully submitted.
pub fn shutdown() -> bool {
    unsafe {
        ExitWindowsEx(
            EWX_SHUTDOWN,
            SHTDN_REASON_MAJOR_APPLICATION | SHTDN_REASON_FLAG_PLANNED,
        )
        .is_ok()
    }
}

/// Restarts Windows using the normal shutdown sequence.
///
/// Returns `true` when the restart request was successfully submitted.
pub fn restart() -> bool {
    unsafe {
        ExitWindowsEx(
            EWX_REBOOT,
            SHTDN_REASON_MAJOR_APPLICATION | SHTDN_REASON_FLAG_PLANNED,
        )
        .is_ok()
    }
}
/// Returns the current battery charge percentage.
///
/// Returns `-1` when Windows cannot provide a valid
/// battery percentage.
pub fn battery_percent() -> i32 {
    let status = power_status();

    if status.unwrap().BatteryLifePercent == 255 {
        -1
    } else {
        status.unwrap().BatteryLifePercent as i32
    }
}

/// Returns whether the battery is currently charging.
///
/// Returns `false` when Windows reports that the battery
/// is not charging.
pub fn battery_charging() -> bool {
    let status = power_status();

    // BatteryFlag bit 3 (value 8) indicates charging.
    (status.unwrap().BatteryFlag & 8) != 0
}

/// Returns whether the computer is connected to AC power.
pub fn battery_on_ac_power() -> bool {
    let status = power_status();

    status.unwrap().ACLineStatus == 1
}

/// Returns whether Windows Battery Saver is enabled.
pub fn battery_saver() -> bool {
    let status = power_status();

    status.unwrap().SystemStatusFlag != 0
}

/// Returns estimated remaining battery time in seconds.
pub fn battery_remaining_seconds() -> i64 {
    let status = match power_status() {
        Some(status) => status,
        None => return -1,
    };

    if status.BatteryLifeTime == u32::MAX {
        return -1;
    }

    status.BatteryLifeTime as i64
}

/// Returns estimated full battery runtime in seconds.
pub fn battery_full_seconds() -> i64 {
    let status = match power_status() {
        Some(status) => status,
        None => return -1,
    };

    if status.BatteryFullLifeTime == u32::MAX {
        return -1;
    }

    status.BatteryFullLifeTime as i64
}

/// Retrieves the Windows power status.
fn power_status() -> Option<SYSTEM_POWER_STATUS> {
    let mut status = SYSTEM_POWER_STATUS::default();

    unsafe {
        GetSystemPowerStatus(&mut status).ok()?;
    }

    Some(status)
}

/// Enables the Windows minimum-power power scheme.
///
/// Returns `true` when Windows successfully activates the scheme.
pub fn enable_power_saving() -> bool {
    let result =
        unsafe { PowerSetActiveScheme(None, Some(&GUID_MIN_POWER_SAVINGS as *const GUID)) };

    result.0 == 0
}

/// Restores the Windows Balanced power scheme.
///
/// Returns `true` when Windows successfully activates the scheme.
pub fn disable_power_saving() -> bool {
    let result = unsafe { PowerSetActiveScheme(None, Some(&GUID_BALANCED as *const GUID)) };

    result.0 == 0
}
