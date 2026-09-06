#![cfg(windows)]

use flame_macro::flame;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use windows::{
    Devices::{
        Bluetooth::{
            Advertisement::{
                BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementType,
                BluetoothLEAdvertisementWatcher, BluetoothLEAdvertisementWatcherStoppedEventArgs,
                BluetoothLEScanningMode,
            },
            BluetoothCacheMode, BluetoothConnectionStatus, BluetoothDevice, BluetoothLEDevice,
        },
        Enumeration::{
            DeviceInformation, DeviceInformationCustomPairing, DeviceInformationUpdate,
            DevicePairingKinds, DevicePairingRequestedEventArgs, DevicePairingResultStatus,
            DeviceWatcher,
        },
        Radios::{Radio, RadioAccessStatus, RadioKind, RadioState},
    },
    Foundation::TypedEventHandler,
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
};

/// Information about a discovered Bluetooth device (Classic or BLE).
#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub address: u64,
    pub address_formatted: String,
    pub name: String,
    pub rssi: i16,
    pub connectable: bool,
    pub advertisement_type: String,
}

/// Formats a 64-bit Bluetooth address as a standard MAC address (e.g., "00:11:22:33:44:55").
fn format_bluetooth_address(address: u64) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        (address >> 40) & 0xFF,
        (address >> 32) & 0xFF,
        (address >> 24) & 0xFF,
        (address >> 16) & 0xFF,
        (address >> 8) & 0xFF,
        address & 0xFF,
    )
}

/// Parses a Bluetooth address from a string (formatted MAC, hex, or decimal).
fn parse_bluetooth_address(input: &str) -> Option<u64> {
    let trimmed = input.trim();

    // Check if it's in MAC address format: "XX:XX:XX:XX:XX:XX" or "XX-XX-XX-XX-XX-XX"
    let clean: String = trimmed.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() == 12 && (trimmed.contains(':') || trimmed.contains('-')) {
        return u64::from_str_radix(&clean, 16).ok();
    }

    // Check if prefixed with "0x"
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok();
    }

    // Check standard decimal or hex
    trimmed
        .parse::<u64>()
        .ok()
        .or_else(|| u64::from_str_radix(trimmed, 16).ok())
}

/// Extracts the REMOTE Bluetooth address from a Windows DeviceInformation ID string.
///
/// Windows Bluetooth IDs have the structure:
/// `"Bluetooth#Bluetooth<LocalAdapterMac>_<RemoteDeviceMac>"`
/// The REMOTE device's MAC is always the LAST 12-hex segment.
fn parse_address_from_device_id(id: &str) -> Option<u64> {
    let parts: Vec<&str> = id.split(['#', '_', '-', '\\', '/']).collect();
    // Search from the end backwards to pick the remote device MAC (not the local adapter MAC)
    for part in parts.iter().rev() {
        let clean: String = part.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() == 12 {
            if let Ok(addr) = u64::from_str_radix(&clean, 16) {
                if addr != 0 {
                    return Some(addr);
                }
            }
        }
    }
    None
}

/// Pairs a device using custom pairing so console / non-UI applications can automatically
/// confirm and accept pairing requests (PIN / ceremony).
fn pair_device_info(info: &DeviceInformation) -> bool {
    let pairing = match info.Pairing() {
        Ok(p) => p,
        Err(_) => return false,
    };

    if pairing.IsPaired().unwrap_or(false) {
        return true;
    }

    // Attempt Custom pairing which allows automatic PIN confirmation without a UWP window
    if let Ok(custom) = pairing.Custom() {
        let handler = TypedEventHandler::<
            DeviceInformationCustomPairing,
            DevicePairingRequestedEventArgs,
        >::new(move |_sender, args| {
            if let Some(args) = args.as_ref() {
                let _ = args.Accept();
            }
            Ok(())
        });

        let _ = custom.PairingRequested(&handler);

        let kinds = DevicePairingKinds::ConfirmOnly
            | DevicePairingKinds::ProvidePin
            | DevicePairingKinds::DisplayPin;

        if let Ok(pair_op) = custom.PairAsync(kinds) {
            if let Ok(res) = pair_op.join() {
                let status = res.Status().unwrap_or(DevicePairingResultStatus::Failed);
                if status == DevicePairingResultStatus::Paired
                    || status == DevicePairingResultStatus::AlreadyPaired
                {
                    return true;
                }
            }
        }
    }

    // Fallback standard PairAsync
    if let Ok(pair_op) = pairing.PairAsync() {
        if let Ok(res) = pair_op.join() {
            let status = res.Status().unwrap_or(DevicePairingResultStatus::Failed);
            return status == DevicePairingResultStatus::Paired
                || status == DevicePairingResultStatus::AlreadyPaired;
        }
    }

    false
}

/// Finds the primary Bluetooth radio adapter on the system.
fn get_bluetooth_radio() -> Option<Radio> {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let async_op = Radio::GetRadiosAsync().ok()?;
    let radios = async_op.join().ok()?;
    let count = radios.Size().ok()?;
    for i in 0..count {
        if let Ok(radio) = radios.GetAt(i) {
            if let Ok(RadioKind::Bluetooth) = radio.Kind() {
                return Some(radio);
            }
        }
    }
    None
}

/// Shared state between Bluetooth manager and advertisement event handlers.
struct SharedScanState {
    devices: Mutex<HashMap<u64, DiscoveredDevice>>,
    is_scanning: Mutex<bool>,
}

/// Provides access to Windows Bluetooth scanning (Classic and BLE), device discovery,
/// connection management, and radio on/off controls.
pub struct Bluetooth {
    le_watcher: Option<BluetoothLEAdvertisementWatcher>,
    device_watchers: Vec<DeviceWatcher>,
    shared_state: Arc<SharedScanState>,
    connected_le_device: Option<BluetoothLEDevice>,
    connected_classic_device: Option<BluetoothDevice>,
}

impl Bluetooth {
    /// Creates a new Windows Bluetooth interface.
    pub fn init() -> Self {
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        Self {
            le_watcher: None,
            device_watchers: Vec::new(),
            shared_state: Arc::new(SharedScanState {
                devices: Mutex::new(HashMap::new()),
                is_scanning: Mutex::new(false),
            }),
            connected_le_device: None,
            connected_classic_device: None,
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Radio On / Off Controls
    // ─────────────────────────────────────────────────────────────

    /// Returns whether the system's Bluetooth radio is turned on / enabled.
    #[flame(rename = "isEnabled")]
    pub fn is_enabled(&self) -> bool {
        match get_bluetooth_radio() {
            Some(radio) => match radio.State() {
                Ok(RadioState::On) => true,
                _ => false,
            },
            None => false,
        }
    }

    /// Turns the system's Bluetooth radio ON.
    ///
    /// Returns `true` if the state change was allowed and successful.
    #[flame(rename = "turnOn")]
    pub fn turn_on(&self) -> bool {
        self.set_enabled(true)
    }

    /// Turns the system's Bluetooth radio OFF.
    ///
    /// Returns `true` if the state change was allowed and successful.
    #[flame(rename = "turnOff")]
    pub fn turn_off(&self) -> bool {
        self.set_enabled(false)
    }

    /// Sets the Bluetooth radio state (on or off).
    ///
    /// Returns `true` if the state change was allowed and successful.
    #[flame(rename = "setEnabled")]
    pub fn set_enabled(&self, enabled: bool) -> bool {
        let radio = match get_bluetooth_radio() {
            Some(r) => r,
            None => return false,
        };

        let _ = Radio::RequestAccessAsync().ok().and_then(|op| op.join().ok());

        let target_state = if enabled {
            RadioState::On
        } else {
            RadioState::Off
        };

        let async_op = match radio.SetStateAsync(target_state) {
            Ok(op) => op,
            Err(_) => return false,
        };

        match async_op.join() {
            Ok(status) => status == RadioAccessStatus::Allowed,
            Err(_) => false,
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Bluetooth Scanning & Discovery (Classic & BLE)
    // ─────────────────────────────────────────────────────────────

    /// Starts active scanning for all nearby Bluetooth devices (phones, computers, audio, and BLE).
    ///
    /// Returns `true` if scanning started successfully, or `false` otherwise.
    #[flame(rename = "startScan")]
    pub fn start_scan(&mut self) -> bool {
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

        // Stop any previous scan
        self.stop_scan();

        let mut any_started = false;

        // 1. Start Bluetooth Low Energy Advertisement Watcher
        if let Ok(watcher) = BluetoothLEAdvertisementWatcher::new() {
            let _ = watcher.SetScanningMode(BluetoothLEScanningMode::Active);

            let shared = Arc::clone(&self.shared_state);
            let received_handler = TypedEventHandler::<
                BluetoothLEAdvertisementWatcher,
                BluetoothLEAdvertisementReceivedEventArgs,
            >::new(move |_watcher, args| {
                if let Some(args) = args.as_ref() {
                    let address = args.BluetoothAddress().unwrap_or(0);
                    if address == 0 {
                        return Ok(());
                    }

                    let rssi = args.RawSignalStrengthInDBm().unwrap_or(0);

                    let adv_type = args
                        .AdvertisementType()
                        .unwrap_or(BluetoothLEAdvertisementType::NonConnectableUndirected);
                    let adv_type_str = match adv_type {
                        BluetoothLEAdvertisementType::ConnectableUndirected => "ConnectableUndirected",
                        BluetoothLEAdvertisementType::ConnectableDirected => "ConnectableDirected",
                        BluetoothLEAdvertisementType::ScannableUndirected => "ScannableUndirected",
                        BluetoothLEAdvertisementType::NonConnectableUndirected => {
                            "NonConnectableUndirected"
                        }
                        BluetoothLEAdvertisementType::ScanResponse => "ScanResponse",
                        BluetoothLEAdvertisementType::Extended => "Extended",
                        _ => "Unknown",
                    };

                    let is_connectable = if let Ok(c) = args.IsConnectable() {
                        c
                    } else {
                        matches!(
                            adv_type,
                            BluetoothLEAdvertisementType::ConnectableUndirected
                                | BluetoothLEAdvertisementType::ConnectableDirected
                        )
                    };

                    let name = args
                        .Advertisement()
                        .ok()
                        .and_then(|adv| adv.LocalName().ok())
                        .map(|h| h.to_string())
                        .unwrap_or_default();

                    if let Ok(mut lock) = shared.devices.lock() {
                        let entry = lock.entry(address).or_insert_with(|| DiscoveredDevice {
                            address,
                            address_formatted: format_bluetooth_address(address),
                            name: name.clone(),
                            rssi,
                            connectable: is_connectable,
                            advertisement_type: adv_type_str.to_string(),
                        });

                        entry.rssi = rssi;
                        if !name.is_empty() {
                            entry.name = name;
                        }
                        entry.connectable = entry.connectable || is_connectable;
                    }
                }
                Ok(())
            });

            if watcher.Received(&received_handler).is_ok() {
                let shared_stopped = Arc::clone(&self.shared_state);
                let stopped_handler = TypedEventHandler::<
                    BluetoothLEAdvertisementWatcher,
                    BluetoothLEAdvertisementWatcherStoppedEventArgs,
                >::new(move |_watcher, _args| {
                    if let Ok(mut is_scanning) = shared_stopped.is_scanning.lock() {
                        *is_scanning = false;
                    }
                    Ok(())
                });

                let _ = watcher.Stopped(&stopped_handler);

                if watcher.Start().is_ok() {
                    self.le_watcher = Some(watcher);
                    any_started = true;
                }
            }
        }

        // 2. Start Windows DeviceWatcher for Classic Bluetooth and BLE
        let selectors = vec![
            BluetoothDevice::GetDeviceSelector().ok(),
            BluetoothDevice::GetDeviceSelectorFromPairingState(false).ok(),
            BluetoothDevice::GetDeviceSelectorFromPairingState(true).ok(),
            BluetoothLEDevice::GetDeviceSelector().ok(),
            BluetoothLEDevice::GetDeviceSelectorFromPairingState(false).ok(),
        ];

        for selector in selectors.into_iter().flatten() {
            if let Ok(dev_watcher) = DeviceInformation::CreateWatcherAqsFilter(&selector) {
                let shared = Arc::clone(&self.shared_state);

                let added_handler = TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
                    move |_watcher, info| {
                        if let Some(info) = info.as_ref() {
                            let mut name = info.Name().map(|n| n.to_string()).unwrap_or_default();
                            let id = info.Id().map(|i| i.to_string()).unwrap_or_default();

                            // Filter out generic Windows loopback / PAN adapter
                            if name.contains("Personal Area Network")
                                || name.contains("Bluetooth Adapter")
                            {
                                return Ok(());
                            }

                            // Query real remote address via FromIdAsync first
                            let mut address = 0u64;
                            let h_id = windows::core::HSTRING::from(&id);
                            if let Ok(op) = BluetoothDevice::FromIdAsync(&h_id) {
                                if let Ok(dev) = op.join() {
                                    address = dev.BluetoothAddress().unwrap_or(0);
                                    if let Ok(n) = dev.Name() {
                                        if !n.is_empty() {
                                            name = n.to_string();
                                        }
                                    }
                                }
                            }
                            if address == 0 {
                                if let Ok(op) = BluetoothLEDevice::FromIdAsync(&h_id) {
                                    if let Ok(dev) = op.join() {
                                        address = dev.BluetoothAddress().unwrap_or(0);
                                        if let Ok(n) = dev.Name() {
                                            if !n.is_empty() {
                                                name = n.to_string();
                                            }
                                        }
                                    }
                                }
                            }

                            if address == 0 {
                                address = parse_address_from_device_id(&id).unwrap_or(0);
                            }

                            if address != 0 {
                                if let Ok(mut lock) = shared.devices.lock() {
                                    let entry =
                                        lock.entry(address).or_insert_with(|| DiscoveredDevice {
                                            address,
                                            address_formatted: format_bluetooth_address(address),
                                            name: name.clone(),
                                            rssi: 0,
                                            connectable: true,
                                            advertisement_type: "Classic".to_string(),
                                        });

                                    if !name.is_empty() {
                                        entry.name = name;
                                    }
                                    entry.connectable = true;
                                }
                            }
                        }
                        Ok(())
                    },
                );

                let _ = dev_watcher.Added(&added_handler);

                let shared_update = Arc::clone(&self.shared_state);
                let updated_handler =
                    TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(
                        move |_watcher, update| {
                            if let Some(update) = update.as_ref() {
                                let id = update.Id().map(|i| i.to_string()).unwrap_or_default();
                                if let Some(address) = parse_address_from_device_id(&id) {
                                    if let Ok(mut lock) = shared_update.devices.lock() {
                                        let entry = lock
                                            .entry(address)
                                            .or_insert_with(|| DiscoveredDevice {
                                                address,
                                                address_formatted: format_bluetooth_address(
                                                    address,
                                                ),
                                                name: String::new(),
                                                rssi: 0,
                                                connectable: true,
                                                advertisement_type: "Classic".to_string(),
                                            });
                                        entry.connectable = true;
                                    }
                                }
                            }
                            Ok(())
                        },
                    );

                let _ = dev_watcher.Updated(&updated_handler);

                if dev_watcher.Start().is_ok() {
                    self.device_watchers.push(dev_watcher);
                    any_started = true;
                }
            }
        }

        if any_started {
            if let Ok(mut is_scanning) = self.shared_state.is_scanning.lock() {
                *is_scanning = true;
            }
        }

        any_started
    }

    /// Stops all active Bluetooth scans.
    ///
    /// Returns `true` if stopped successfully.
    #[flame(rename = "stopScan")]
    pub fn stop_scan(&mut self) -> bool {
        if let Some(watcher) = self.le_watcher.take() {
            let _ = watcher.Stop();
        }

        for watcher in self.device_watchers.drain(..) {
            let _ = watcher.Stop();
        }

        if let Ok(mut is_scanning) = self.shared_state.is_scanning.lock() {
            *is_scanning = false;
        }

        true
    }

    /// Returns whether a Bluetooth scan is actively running.
    #[flame(rename = "isScanning")]
    pub fn is_scanning(&self) -> bool {
        self.shared_state
            .is_scanning
            .lock()
            .map(|s| *s)
            .unwrap_or(false)
    }

    /// Clears the list of discovered devices.
    #[flame(rename = "clearDevices")]
    pub fn clear_devices(&mut self) -> bool {
        if let Ok(mut devices) = self.shared_state.devices.lock() {
            devices.clear();
            true
        } else {
            false
        }
    }

    /// Returns the number of discovered Bluetooth devices.
    #[flame(rename = "deviceCount")]
    pub fn device_count(&self) -> u32 {
        self.shared_state
            .devices
            .lock()
            .map(|d| d.len() as u32)
            .unwrap_or(0)
    }

    /// Returns a list of all discovered devices as a JSON string.
    ///
    /// Each device object contains `address`, `address_raw`, `name`, `rssi`, `connectable`,
    /// and `advertisement_type`.
    pub fn devices(&self) -> String {
        let lock = match self.shared_state.devices.lock() {
            Ok(l) => l,
            Err(_) => return "[]".to_string(),
        };

        let mut items = Vec::new();
        for dev in lock.values() {
            let mut name = dev.name.clone();

            // If name is empty, try resolving friendly name from system
            if name.is_empty() {
                if let Ok(op) = BluetoothDevice::FromBluetoothAddressAsync(dev.address) {
                    if let Ok(classic) = op.join() {
                        if let Ok(n) = classic.Name() {
                            name = n.to_string();
                        }
                    }
                }
                if name.is_empty() {
                    if let Ok(op) = BluetoothLEDevice::FromBluetoothAddressAsync(dev.address) {
                        if let Ok(le) = op.join() {
                            if let Ok(n) = le.Name() {
                                name = n.to_string();
                            }
                        }
                    }
                }
            }

            let display_name = if name.is_empty() {
                format!("Device ({})", dev.address_formatted)
            } else {
                name
            };

            let escaped_name = display_name
                .replace('\\', "\\\\")
                .replace('"', "\\\"");

            items.push(format!(
                r#"{{"address":"{}","address_raw":{},"name":"{}","rssi":{},"connectable":{},"advertisement_type":"{}"}}"#,
                dev.address_formatted,
                dev.address,
                escaped_name,
                dev.rssi,
                dev.connectable,
                dev.advertisement_type
            ));
        }

        format!("[{}]", items.join(","))
    }

    /// Returns the name of a discovered device by index (0-based).
    #[flame(rename = "getDeviceName")]
    pub fn get_device_name(&self, index: i64) -> String {
        let lock = match self.shared_state.devices.lock() {
            Ok(l) => l,
            Err(_) => return String::new(),
        };

        if index < 0 || index as usize >= lock.len() {
            return String::new();
        }

        lock.values()
            .nth(index as usize)
            .map(|d| d.name.clone())
            .unwrap_or_default()
    }

    /// Returns the MAC address of a discovered device by index (0-based).
    #[flame(rename = "getDeviceAddress")]
    pub fn get_device_address(&self, index: i64) -> String {
        let lock = match self.shared_state.devices.lock() {
            Ok(l) => l,
            Err(_) => return String::new(),
        };

        if index < 0 || index as usize >= lock.len() {
            return String::new();
        }

        lock.values()
            .nth(index as usize)
            .map(|d| d.address_formatted.clone())
            .unwrap_or_default()
    }

    /// Returns the RSSI signal strength of a discovered device by index (0-based).
    #[flame(rename = "getDeviceRssi")]
    pub fn get_device_rssi(&self, index: i64) -> i64 {
        let lock = match self.shared_state.devices.lock() {
            Ok(l) => l,
            Err(_) => return 0,
        };

        if index < 0 || index as usize >= lock.len() {
            return 0;
        }

        lock.values()
            .nth(index as usize)
            .map(|d| d.rssi as i64)
            .unwrap_or(0)
    }

    /// Returns whether a discovered device is connectable by index (0-based).
    #[flame(rename = "isDeviceConnectable")]
    pub fn is_device_connectable(&self, index: i64) -> bool {
        let lock = match self.shared_state.devices.lock() {
            Ok(l) => l,
            Err(_) => return false,
        };

        if index < 0 || index as usize >= lock.len() {
            return false;
        }

        lock.values()
            .nth(index as usize)
            .map(|d| d.connectable)
            .unwrap_or(false)
    }

    // ─────────────────────────────────────────────────────────────
    // Connection Management (Classic & BLE)
    // ─────────────────────────────────────────────────────────────

    /// Connects to a Bluetooth device (Classic or BLE) using its address
    /// (MAC format "XX:XX:XX:XX:XX:XX", hex, or decimal) or name.
    ///
    /// Automatically pairs with the device if it is not yet paired.
    /// Returns `true` when the device is successfully resolved and connected/paired.
    pub fn connect(&mut self, target: String) -> bool {
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

        let address = if let Some(addr) = parse_bluetooth_address(&target) {
            addr
        } else {
            // Try looking up device by name in discovered list
            let lock = match self.shared_state.devices.lock() {
                Ok(l) => l,
                Err(_) => return false,
            };

            let found = lock
                .values()
                .find(|d| d.name.eq_ignore_ascii_case(target.trim()));

            match found {
                Some(d) => d.address,
                None => return false,
            }
        };

        if address == 0 {
            return false;
        }

        self.disconnect();

        // 1. Try Classic Bluetooth (phones, headphones, speakers, PCs)
        if let Ok(async_op) = BluetoothDevice::FromBluetoothAddressAsync(address) {
            if let Ok(dev) = async_op.join() {
                if let Ok(info) = dev.DeviceInformation() {
                    let _ = pair_device_info(&info);
                }

                // Query RFCOMM services with Uncached mode to physically establish the connection with the phone
                let _ = dev
                    .GetRfcommServicesWithCacheModeAsync(BluetoothCacheMode::Uncached)
                    .or_else(|_| dev.GetRfcommServicesAsync())
                    .ok()
                    .and_then(|op| op.join().ok());

                self.connected_classic_device = Some(dev);
                return true;
            }
        }

        // 2. Try BLE (smartwatches, beacons, peripherals)
        if let Ok(async_op) = BluetoothLEDevice::FromBluetoothAddressAsync(address) {
            if let Ok(dev) = async_op.join() {
                if let Ok(info) = dev.DeviceInformation() {
                    let _ = pair_device_info(&info);
                }

                // Force GATT connection
                let _ = dev
                    .GetGattServicesWithCacheModeAsync(BluetoothCacheMode::Uncached)
                    .or_else(|_| dev.GetGattServicesAsync())
                    .ok()
                    .and_then(|op| op.join().ok());

                self.connected_le_device = Some(dev);
                return true;
            }
        }

        false
    }

    /// Explicitly pairs with a Bluetooth device by address or name.
    ///
    /// Returns `true` if pairing was initiated and succeeded, or if already paired.
    #[flame(rename = "pair")]
    pub fn pair(&mut self, target: String) -> bool {
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

        let address = if let Some(addr) = parse_bluetooth_address(&target) {
            addr
        } else {
            let lock = match self.shared_state.devices.lock() {
                Ok(l) => l,
                Err(_) => return false,
            };
            lock.values()
                .find(|d| d.name.eq_ignore_ascii_case(target.trim()))
                .map(|d| d.address)
                .unwrap_or(0)
        };

        if address == 0 {
            return false;
        }

        if let Ok(async_op) = BluetoothDevice::FromBluetoothAddressAsync(address) {
            if let Ok(dev) = async_op.join() {
                if let Ok(info) = dev.DeviceInformation() {
                    return pair_device_info(&info);
                }
            }
        }

        if let Ok(async_op) = BluetoothLEDevice::FromBluetoothAddressAsync(address) {
            if let Ok(dev) = async_op.join() {
                if let Ok(info) = dev.DeviceInformation() {
                    return pair_device_info(&info);
                }
            }
        }

        false
    }

    /// Returns whether the currently selected or connected device is paired with Windows.
    #[flame(rename = "isPaired")]
    pub fn is_paired(&self) -> bool {
        if let Some(classic) = &self.connected_classic_device {
            if let Ok(info) = classic.DeviceInformation() {
                if let Ok(pairing) = info.Pairing() {
                    return pairing.IsPaired().unwrap_or(false);
                }
            }
        }

        if let Some(le) = &self.connected_le_device {
            if let Ok(info) = le.DeviceInformation() {
                if let Ok(pairing) = info.Pairing() {
                    return pairing.IsPaired().unwrap_or(false);
                }
            }
        }

        false
    }

    /// Returns whether a Bluetooth device is currently connected.
    #[flame(rename = "isConnected")]
    pub fn is_connected(&self) -> bool {
        if let Some(classic) = &self.connected_classic_device {
            if matches!(
                classic.ConnectionStatus(),
                Ok(BluetoothConnectionStatus::Connected)
            ) {
                return true;
            }
        }

        if let Some(le) = &self.connected_le_device {
            if matches!(
                le.ConnectionStatus(),
                Ok(BluetoothConnectionStatus::Connected)
            ) {
                return true;
            }
        }

        false
    }

    /// Returns the current connection status as a string:
    /// `"connected"`, `"paired"`, `"disconnected"`, or `"unknown"`.
    #[flame(rename = "connectionStatus")]
    pub fn connection_status(&self) -> String {
        if let Some(classic) = &self.connected_classic_device {
            if matches!(
                classic.ConnectionStatus(),
                Ok(BluetoothConnectionStatus::Connected)
            ) {
                return "connected".to_string();
            }
            if self.is_paired() {
                return "paired".to_string();
            }
            return "disconnected".to_string();
        }

        if let Some(le) = &self.connected_le_device {
            if matches!(
                le.ConnectionStatus(),
                Ok(BluetoothConnectionStatus::Connected)
            ) {
                return "connected".to_string();
            }
            if self.is_paired() {
                return "paired".to_string();
            }
            return "disconnected".to_string();
        }

        "disconnected".to_string()
    }

    /// Returns the name of the currently connected device.
    #[flame(rename = "connectedDeviceName")]
    pub fn connected_device_name(&self) -> String {
        if let Some(classic) = &self.connected_classic_device {
            if let Ok(name) = classic.Name() {
                return name.to_string();
            }
        }

        if let Some(le) = &self.connected_le_device {
            if let Ok(name) = le.Name() {
                return name.to_string();
            }
        }

        String::new()
    }

    /// Returns the formatted address of the currently connected device.
    #[flame(rename = "connectedDeviceAddress")]
    pub fn connected_device_address(&self) -> String {
        if let Some(classic) = &self.connected_classic_device {
            if let Ok(addr) = classic.BluetoothAddress() {
                return format_bluetooth_address(addr);
            }
        }

        if let Some(le) = &self.connected_le_device {
            if let Ok(addr) = le.BluetoothAddress() {
                return format_bluetooth_address(addr);
            }
        }

        String::new()
    }

    /// Disconnects the currently connected Bluetooth device.
    pub fn disconnect(&mut self) -> bool {
        let mut disconnected = false;

        if let Some(device) = self.connected_classic_device.take() {
            let _ = device.Close();
            disconnected = true;
        }

        if let Some(device) = self.connected_le_device.take() {
            let _ = device.Close();
            disconnected = true;
        }

        disconnected
    }
}
