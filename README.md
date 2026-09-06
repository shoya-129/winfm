# winfm

`winfm` is a native Windows package for [Flame](https://github.com/shoya-129/flame) that provides a simple, high-level API for interacting with Windows system features.

It gives Flame applications access to system information, machine controls, battery status, power management, clipboard operations, master audio volume, and Bluetooth scanning and connectivity without exposing the underlying Windows APIs.

## Features

* 🖥️ System information and machine controls
* 🔋 Battery status and power management
* 📋 System clipboard access
* 🔊 Master audio volume and mute control
* 📶 Bluetooth LE device scanning, discovery, and connection management
* ⚡ Windows power-saving controls
* 🪟 Native Windows integration

The package keeps all Windows-specific implementation details behind the package boundary. Flame applications interact with clean, high-level APIs rather than Win32 handles, COM interfaces, Windows power APIs, or native runtime handles directly.

## Installation

Add `winfm` to your Flame project:

```bash
fmp add https://github.com/shoya-129/winfm
```

### Import

All features are available directly from the standard `winfm` package:

```flame
import winfm
```

---

## Audio (Volume)

The audio API provides control over the Windows master audio volume and mute state.

### Getting the Audio Interface

Call `winfm.audio()` and store the interface in a variable:

```flame
import winfm

let v = winfm.audio()
```

### Read Master Volume

```flame
import winfm

let v = winfm.audio()

println(v.percent())
```

The returned value is an integer percentage from `0` to `100`.

### Set Master Volume

```flame
import winfm

let v = winfm.audio()

v.set(50)
```

The value is automatically clamped to the supported `0` to `100` range.

### Read Mute State

```flame
import winfm

let v = winfm.audio()

println(v.muted())
```

### Mute and Unmute

Mute the audio:

```flame
import winfm

let v = winfm.audio()

v.setMuted(true)
```

Unmute the audio:

```flame
import winfm

let v = winfm.audio()

v.setMuted(false)
```

### Audio API Summary

| Method                | Return Type | Description                              |
| --------------------- | ----------- | ---------------------------------------- |
| `winfm.audio()`       | `Volume`    | Creates the audio volume interface       |
| `v.percent()`         | `Int`       | Returns the current master volume (0–100)|
| `v.set(percent)`      | `Bool`      | Sets the master volume (clamped 0–100)   |
| `v.muted()`           | `Bool`      | Returns whether the volume is muted      |
| `v.setMuted(boolean)` | `Bool`      | Mutes (`true`) or unmutes (`false`) audio|

### Audio Example

```flame
import winfm

let v = winfm.audio()

println("Current volume: " + v.percent().toString() + "%")
println("Is muted: " + v.muted().toString())

// Set volume to 65%
v.set(65)
println("New volume: " + v.percent().toString() + "%")

// Toggle mute
v.setMuted(true)
println("Muted: " + v.muted().toString())
v.setMuted(false)
```

---

## Clipboard

The clipboard API provides access to read, write, and clear the Windows system clipboard.

### Getting the Clipboard Interface

Call `winfm.clipboard()` and store the interface in a variable:

```flame
import winfm

let c = winfm.clipboard()
```

### Read Clipboard Text

```flame
import winfm

let c = winfm.clipboard()

println(c.get())
```

### Set Clipboard Text

```flame
import winfm

let c = winfm.clipboard()

c.set("Hello from Flame!")
```

### Clear Clipboard

```flame
import winfm

let c = winfm.clipboard()

c.clear()
```

### Clipboard API Summary

| Method              | Return Type | Description                                   |
| ------------------- | ----------- | --------------------------------------------- |
| `winfm.clipboard()` | `Clipboard` | Creates the clipboard interface               |
| `c.get()`           | `String`    | Returns the current Unicode text from clipboard|
| `c.set(text)`       | `Bool`      | Replaces clipboard contents with the text     |
| `c.clear()`         | `Bool`      | Clears all content from the system clipboard  |

### Clipboard Example

```flame
import winfm

let c = winfm.clipboard()

// Write text to clipboard
c.set("Hello from Flame!")

// Read and print clipboard text
println("Clipboard contains: " + c.get())

// Clear clipboard
c.clear()
```

---

## Bluetooth

The Bluetooth API provides control over the system's Bluetooth radio (turning it on/off, checking status), scanning for nearby Bluetooth devices, discovering device details (names, MAC addresses, RSSI signal strength, advertisement types, and connectability), as well as connecting and disconnecting from devices.

### Getting the Bluetooth Interface

Call `winfm.bluetooth()` and store the interface in a variable:

```flame
import winfm

let bt = winfm.bluetooth()
```

### Turning Bluetooth On and Off

Check if Bluetooth radio is enabled:

```flame
let enabled = bt.isEnabled()
println("Bluetooth enabled: " + enabled.toString())
```

Turn Bluetooth ON:

```flame
let success = bt.turnOn()
println("Turned on: " + success.toString())
```

Turn Bluetooth OFF:

```flame
let success = bt.turnOff()
println("Turned off: " + success.toString())
```

Or set the state directly:

```flame
bt.setEnabled(true)  // turn on
bt.setEnabled(false) // turn off
```

### Scanning for Nearby Devices

Start active background scanning:

```flame
import winfm

let bt = winfm.bluetooth()

bt.startScan()
println("Scanning active: " + bt.isScanning().toString())
```

Stop scanning when done:

```flame
bt.stopScan()
```

### Inspecting Discovered Devices

Get the full device list as a JSON string:

```flame
let listJson = bt.devices()
println(listJson)
```

Each device in the JSON array contains:
* `address`: Formatted MAC address (e.g., `"E4:5F:01:23:45:67"`)
* `address_raw`: 64-bit integer address
* `name`: Device local name (if advertised)
* `rssi`: Signal strength in dBm
* `connectable`: `true` if the device is connectable
* `advertisement_type`: Advertisement type string (`"ConnectableUndirected"`, `"ConnectableDirected"`, etc.)

You can also query devices individually by index (`0` to `deviceCount() - 1`):

```flame
let count = bt.deviceCount()
println("Discovered devices: " + count.toString())

let name = bt.getDeviceName(0)
let addr = bt.getDeviceAddress(0)
let rssi = bt.getDeviceRssi(0)
let connectable = bt.isDeviceConnectable(0)

println("Device 0: " + name + " [" + addr + "] RSSI: " + rssi.toString() + " Connectable: " + connectable.toString())
```

Clear discovered devices cache:

```flame
bt.clearDevices()
```

### Connecting to a Device

Connect to a Bluetooth device by its formatted MAC address (`"XX:XX:XX:XX:XX:XX"`), hex address, or by discovered device name:

```flame
let success = bt.connect("E4:5F:01:23:45:67")
if success {
    println("Connected to: " + bt.connectedDeviceName())
    println("Address: " + bt.connectedDeviceAddress())
    println("Status: " + bt.connectionStatus())
}
```

Check connection status:

```flame
println(bt.isConnected())        // Returns Bool
println(bt.connectionStatus())    // Returns "connected" or "disconnected"
```

### Disconnecting

```flame
bt.disconnect()
```

### Bluetooth API Summary

| Method                              | Return Type | Description                                                        |
| ----------------------------------- | ----------- | ------------------------------------------------------------------ |
| `winfm.bluetooth()`                 | `Bluetooth` | Creates the Bluetooth interface                                    |
| `bt.isEnabled()`                    | `Bool`      | Returns whether Bluetooth radio is turned on / enabled             |
| `bt.turnOn()`                       | `Bool`      | Turns the system's Bluetooth radio ON                              |
| `bt.turnOff()`                      | `Bool`      | Turns the system's Bluetooth radio OFF                             |
| `bt.setEnabled(boolean)`            | `Bool`      | Sets the Bluetooth radio state to on (`true`) or off (`false`)     |
| `bt.startScan()`                    | `Bool`      | Starts active BLE advertisement scanning                           |
| `bt.stopScan()`                     | `Bool`      | Stops the active BLE advertisement scan                            |
| `bt.isScanning()`                   | `Bool`      | Returns whether scanning is currently active                       |
| `bt.deviceCount()`                  | `Int`       | Returns the total number of discovered devices                     |
| `bt.devices()`                      | `String`    | Returns all discovered devices formatted as a JSON array string    |
| `bt.getDeviceName(index)`           | `String`    | Returns the name of the device at `index`                          |
| `bt.getDeviceAddress(index)`        | `String`    | Returns the formatted MAC address of the device at `index`         |
| `bt.getDeviceRssi(index)`           | `Int`       | Returns the signal strength (RSSI) of the device at `index`        |
| `bt.isDeviceConnectable(index)`     | `Bool`      | Returns whether the device at `index` is connectable               |
| `bt.clearDevices()`                 | `Bool`      | Clears the discovered device cache                                 |
| `bt.pair(target)`                   | `Bool`      | Initiates pairing with a device by MAC address or name             |
| `bt.isPaired()`                     | `Bool`      | Returns whether the current device is paired with Windows          |
| `bt.connect(target)`                | `Bool`      | Connects/pairs to a device by MAC address or name                  |
| `bt.disconnect()`                   | `Bool`      | Disconnects the currently connected device                         |
| `bt.isConnected()`                  | `Bool`      | Returns whether a Bluetooth device is currently connected          |
| `bt.connectionStatus()`             | `String`    | Returns connection status (`"connected"`, `"paired"`, etc.)        |
| `bt.connectedDeviceName()`          | `String`    | Returns the name of the connected device                           |
| `bt.connectedDeviceAddress()`       | `String`    | Returns the MAC address of the connected device                    |

---

## System

The `system` API provides information about the Windows computer and controls for the Windows session.

```flame
import winfm
```

### System Information

```flame
import winfm

let hostname = winfm.system.hostname()
let username = winfm.system.username()
let cpus = winfm.system.cpus()
let uptime = winfm.system.uptime()

println("Hostname: " + hostname)
println("Username: " + username)
println("CPUs: " + cpus.toString())
println("Uptime: " + uptime.h.toString() + "h " + uptime.min.toString() + "m " + uptime.sec.toString() + "s")
```

### System API Summary

| API                       | Return Type | Description                           |
| ------------------------- | ----------- | ------------------------------------- |
| `winfm.system.hostname()` | `String`    | Returns the Windows computer hostname |
| `winfm.system.username()` | `String`    | Returns the current logged-in username|
| `winfm.system.cpus()`     | `Int`       | Returns the number of logical CPUs    |
| `winfm.system.uptime()`   | `Formula`   | Returns system uptime (`h`, `min`, `sec`)|

### Machine Controls

Lock the current Windows session:

```flame
import winfm

winfm.system.lock()
```

Put Windows into sleep mode:

```flame
import winfm

winfm.system.sleep()
```

Restart Windows:

```flame
import winfm

winfm.system.restart()
```

Shut down Windows:

```flame
import winfm

winfm.system.shutdown()
```

---

## Battery

The `battery` API provides real-time information about battery percentage, charging state, AC power, and battery saver settings.

```flame
import winfm
```

### Battery Status

```flame
import winfm

let b = winfm.battery.status()

println("Percent: " + b.percent.toString() + "%")
println("Charging: " + b.charging.toString())
println("On AC Power: " + b.onAcPower.toString())
println("Battery Saver: " + b.saver.toString())

if b.remaining.available {
    println(
        "Remaining runtime: " +
        b.remaining.h.toString() + "h " +
        b.remaining.min.toString() + "m " +
        b.remaining.sec.toString() + "s"
    )
}
```

The returned `Battery` value contains:

| Property    | Type       | Description                                   |
| ----------- | ---------- | --------------------------------------------- |
| `percent`   | `Int`      | Current battery percentage (0–100)            |
| `charging`  | `Bool`     | Whether the battery is actively charging      |
| `onAcPower` | `Bool`     | Whether the machine is connected to AC power  |
| `saver`     | `Bool`     | Whether Windows battery saver mode is active  |
| `remaining` | `Duration` | Estimated remaining battery runtime           |
| `full`      | `Duration` | Estimated full battery charge runtime         |

### Battery Runtime Duration

A `Duration` contains:

| Property    | Type   | Description                                 |
| ----------- | ------ | ------------------------------------------- |
| `h`         | `Int`  | Hours                                       |
| `min`       | `Int`  | Minutes                                     |
| `sec`       | `Int`  | Seconds                                     |
| `available` | `Bool` | Whether Windows provided a valid estimate   |

### Battery Saver Controls

Check if battery saver is enabled:

```flame
println(winfm.battery.batterySaver())
```

Toggle battery saver:

```flame
let changed = winfm.battery.batterySaverToggle()
println("Saver toggled: " + changed.toString())
```

*Note: The toggle automatically avoids toggling power-saving mode when connected to AC power or while charging.*

### Battery API Summary

| API                                  | Return Type | Description                                |
| ------------------------------------ | ----------- | ------------------------------------------ |
| `winfm.battery.status()`             | `Battery`   | Returns complete battery status            |
| `winfm.battery.batterySaver()`       | `Bool`      | Returns current battery saver active state |
| `winfm.battery.batterySaverToggle()` | `Bool`      | Toggles power-saving mode when allowed     |

---

## Complete Example

Here is an end-to-end example demonstrating all modules together using standard `import winfm`:

```flame
import winfm

println("=== System ===")
println("Hostname: " + winfm.system.hostname())
println("Username: " + winfm.system.username())
println("CPUs: " + winfm.system.cpus().toString())

println("=== Battery ===")
let b = winfm.battery.status()
println("Battery: " + b.percent.toString() + "%")
println("Charging: " + b.charging.toString())
println("AC Power: " + b.onAcPower.toString())
println("Battery Saver: " + b.saver.toString())

println("=== Audio ===")
let v = winfm.audio()
println("Volume: " + v.percent().toString() + "%")
println("Muted: " + v.muted().toString())
v.set(50)
v.setMuted(false)

println("=== Clipboard ===")
let c = winfm.clipboard()
c.set("Hello from Flame & winfm!")
println("Clipboard: " + c.get())

println("=== Bluetooth ===")
let bt = winfm.bluetooth()
println("Bluetooth enabled: " + bt.isEnabled().toString())
bt.startScan()
println("Scanning: " + bt.isScanning().toString())
println("Discovered devices: " + bt.devices())
bt.stopScan()
```

---

## Platform Support

`winfm` is designed specifically for **Windows 10 and Windows 11**.

---

## Repository

[github.com/shoya-129/winfm](https://github.com/shoya-129/winfm)
