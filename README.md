# winfm

`winfm` is a native Windows package for [Flame](https://github.com/shoya-129/flame) that provides a simple, high-level API for interacting with Windows system features.

It gives Flame applications access to system information, machine controls, battery status, power management, clipboard operations, and master audio volume without exposing the underlying Windows APIs.

## Features

* 🖥️ System information and machine controls
* 🔋 Battery status and power management
* 📋 Clipboard access
* 🔊 Master volume control
* ⚡ Windows power-saving controls
* 🪟 Native Windows integration

The package keeps Windows-specific implementation details behind the package boundary. Flame applications interact with high-level APIs rather than Win32 handles, COM interfaces, Windows power APIs, or clipboard memory directly.

## Installation

Add `winfm` to your Flame project:

```bash
flame add https://github.com/shoya-129/winfm
```

### Standard package import

The high-level exported APIs are available through:

```flame
import winfm
```

Use this import for:

* `winfm.system`
* `winfm.battery`

### Native API import

Clipboard and volume are native interfaces and require:

```flame
import native.winfm
```

Use this import when working with:

* `winfm.Clipboard`
* `winfm.Volume`

## System

The `system` API provides information about the current Windows machine and controls for the current Windows session.

Import it with:

```flame
import winfm
```

### System information

```flame
import winfm

let hostname = winfm.system.hostname()
let username = winfm.system.username()
let cpus = winfm.system.cpus()
let uptime = winfm.system.uptime()

println(hostname)
println(username)
println(cpus)
println(uptime)
```

### System API

| API                       | Description                           |
| ------------------------- | ------------------------------------- |
| `winfm.system.hostname()` | Returns the Windows computer hostname |
| `winfm.system.username()` | Returns the current Windows username  |
| `winfm.system.cpus()`     | Returns the number of logical CPUs    |
| `winfm.system.uptime()`   | Returns system uptime                 |

### Machine controls

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

`restart()` and `shutdown()` request the corresponding Windows system operation.

### System example

```flame
import winfm

println("=== System ===")

println("Hostname: " + winfm.system.hostname())
println("Username: " + winfm.system.username())
println("CPUs: " + winfm.system.cpus().toString())
println("Uptime: " + winfm.system.uptime().toString())

winfm.system.lock()
```

## Battery

The `battery` API provides a high-level view of the current battery and power state.

Import it with:

```flame
import winfm
```

### Battery status

```flame
import winfm

let battery = winfm.battery.status()

println(battery.percent)
println(battery.charging)
println(battery.onAcPower)
println(battery.saver)
println(battery.power_saving)
```

The returned `Battery` value contains:

| Property       | Description                                   |
| -------------- | --------------------------------------------- |
| `percent`      | Current battery percentage                    |
| `charging`     | Whether the battery is charging               |
| `onAcPower`    | Whether the computer is connected to AC power |
| `saver`        | Whether Windows battery saver is active       |
| `power_saving` | Current power-saving state                    |
| `remaining`    | Estimated remaining battery runtime           |
| `full`         | Estimated full battery runtime                |

### Battery runtime

Battery runtime is represented as a `Duration` value.

```flame
import winfm

let battery = winfm.battery.status()

let remaining = battery.remaining

println(remaining.h)
println(remaining.min)
println(remaining.sec)
println(remaining.available)
```

A `Duration` contains:

| Property    | Description                                 |
| ----------- | ------------------------------------------- |
| `h`         | Hours                                       |
| `min`       | Minutes                                     |
| `sec`       | Seconds                                     |
| `available` | Whether Windows provided a runtime estimate |

For example:

```flame
import winfm

let battery = winfm.battery.status()

if battery.remaining.available {
    println(
        "Remaining: " +
        battery.remaining.h.toString() +
        "h " +
        battery.remaining.min.toString() +
        "m " +
        battery.remaining.sec.toString() +
        "s"
    )
}
```

The same structure is available through `battery.full`.

Battery runtime estimates are provided by Windows and may be unavailable on some systems or power states. When an estimate is unavailable, `available` is `false`.

### Battery saver

Read the current battery saver state:

```flame
import winfm

println(winfm.battery.batterySaver())
```

Toggle power saving:

```flame
import winfm

let changed = winfm.battery.batterySaverToggle()

println(changed)
```

The toggle avoids changing the power-saving mode while the machine is connected to AC power or while the battery is charging.

### Battery API

| API                                  | Description                                |
| ------------------------------------ | ------------------------------------------ |
| `winfm.battery.status()`             | Returns the complete battery status        |
| `winfm.battery.batterySaver()`       | Returns the current battery saver state    |
| `winfm.battery.batterySaverToggle()` | Toggles the power-saving mode when allowed |

### Battery example

```flame
import winfm

let battery = winfm.battery.status()

println("=== Battery ===")

println("Battery: " + battery.percent.toString() + "%")
println("Charging: " + battery.charging.toString())
println("AC Power: " + battery.onAcPower.toString())
println("Battery Saver: " + battery.saver.toString())
println("Power Saving: " + battery.power_saving.toString())

if battery.remaining.available {
    println(
        "Remaining: " +
        battery.remaining.h.toString() +
        "h " +
        battery.remaining.min.toString() +
        "m " +
        battery.remaining.sec.toString() +
        "s"
    )
}

if battery.full.available {
    println(
        "Full Runtime: " +
        battery.full.h.toString() +
        "h " +
        battery.full.min.toString() +
        "m " +
        battery.full.sec.toString() +
        "s"
    )
}
```

## Clipboard

The `Clipboard` API provides access to the Windows system clipboard.

Clipboard is exposed through the native package interface, so use:

```flame
import native.winfm
```

### Create a clipboard interface

```flame
import native.winfm

let clipboard = winfm.Clipboard.init()
```

The clipboard interface is lightweight. Windows owns the actual system clipboard.

### Read clipboard text

```flame
import native.winfm

let clipboard = winfm.Clipboard.init()

println(clipboard.get())
```

### Set clipboard text

```flame
import native.winfm

let clipboard = winfm.Clipboard.init()

clipboard.set("Hello from Flame")
```

### Clear clipboard

```flame
import native.winfm

let clipboard = winfm.Clipboard.init()

clipboard.clear()
```

### Clipboard API

| API                      | Description                        |
| ------------------------ | ---------------------------------- |
| `winfm.Clipboard.init()` | Creates a clipboard interface      |
| `clipboard.get()`        | Returns the current clipboard text |
| `clipboard.set(text)`    | Replaces the clipboard text        |
| `clipboard.clear()`      | Clears the clipboard               |

### Clipboard example

```flame
import native.winfm

let clipboard = winfm.Clipboard.init()

println(clipboard.get())

let result = clipboard.set("Hello from Flame")

println(result)
println(clipboard.get())

clipboard.clear()
```

## Volume

The `Volume` API provides control over the Windows master audio volume.

Volume is exposed through the native package interface, so use:

```flame
import native.winfm
```

### Create a volume interface

```flame
import native.winfm

let volume = winfm.Volume.init()
```

### Read volume

```flame
import native.winfm

let volume = winfm.Volume.init()

println(volume.percent())
```

The returned value is a percentage from `0` to `100`.

### Set volume

```flame
import native.winfm

let volume = winfm.Volume.init()

volume.set(50)
```

The value is clamped to the supported `0` to `100` range.

### Read mute state

```flame
import native.winfm

let volume = winfm.Volume.init()

println(volume.muted())
```

### Mute and unmute

Mute the system:

```flame
import native.winfm

let volume = winfm.Volume.init()

volume.set_muted(true)
```

Unmute the system:

```flame
import native.winfm

let volume = winfm.Volume.init()

volume.set_muted(false)
```

### Volume API

| API                       | Description                              |
| ------------------------- | ---------------------------------------- |
| `winfm.Volume.init()`     | Creates a volume interface               |
| `volume.percent()`        | Returns the current master volume        |
| `volume.set(percent)`     | Sets the master volume from `0` to `100` |
| `volume.muted()`          | Returns whether the volume is muted      |
| `volume.set_muted(value)` | Mutes or unmutes the master volume       |

### Volume example

```flame
import native.winfm

let volume = winfm.Volume.init()

println("Volume: " + volume.percent().toString())
println("Muted: " + volume.muted().toString())

volume.set(50)

println("New Volume: " + volume.percent().toString())

volume.set_muted(false)
```

## API overview

The public Flame API is intentionally divided into high-level package APIs and native interfaces.

### Standard APIs

Import with:

```flame
import winfm
```

System:

```text
winfm.system.hostname()
winfm.system.username()
winfm.system.cpus()
winfm.system.uptime()

winfm.system.lock()
winfm.system.sleep()
winfm.system.restart()
winfm.system.shutdown()
```

Battery:

```text
winfm.battery.status()
winfm.battery.batterySaver()
winfm.battery.batterySaverToggle()
```

### Native APIs

Import with:

```flame
import native.winfm
```

Clipboard:

```text
winfm.Clipboard.init()

clipboard.get()
clipboard.set(text)
clipboard.clear()
```

Volume:

```text
winfm.Volume.init()

volume.percent()
volume.set(percent)
volume.muted()
volume.set_muted(value)
```

This separation keeps the commonly used system and battery functionality exposed through a simple package API while native stateful interfaces such as clipboard and volume remain explicitly native.

## Complete example

The following example demonstrates the main `winfm` APIs together:

```flame
import winfm
import native.winfm

println("=== System ===")

println("Hostname: " + winfm.system.hostname())
println("Username: " + winfm.system.username())
println("CPUs: " + winfm.system.cpus().toString())
println("Uptime: " + winfm.system.uptime().toString())

println("=== Battery ===")

let battery = winfm.battery.status()

println("Battery: " + battery.percent.toString() + "%")
println("Charging: " + battery.charging.toString())
println("AC Power: " + battery.onAcPower.toString())
println("Battery Saver: " + battery.saver.toString())
println("Power Saving: " + battery.power_saving.toString())

if battery.remaining.available {
    println(
        "Remaining: " +
        battery.remaining.h.toString() +
        "h " +
        battery.remaining.min.toString() +
        "m " +
        battery.remaining.sec.toString() +
        "s"
    )
}

println("=== Clipboard ===")

let clipboard = winfm.Clipboard.init()

clipboard.set("Hello from Flame")

println(clipboard.get())

println("=== Volume ===")

let volume = winfm.Volume.init()

println("Volume: " + volume.percent().toString())
println("Muted: " + volume.muted().toString())

volume.set(50)

println("New Volume: " + volume.percent().toString())
```

## Notice

`winfm` is a **Windows-only** native package.

It requires:

* Windows

The package is not intended for Linux or macOS.

## Design

`winfm` keeps the Windows-specific implementation behind the package boundary.

A Flame application does not need to interact directly with Windows APIs such as:

* Win32 system APIs
* Windows power-management APIs
* Windows clipboard handles
* Windows audio APIs
* COM interfaces

Instead, the application uses the high-level `winfm` API:

```text
Flame application
       │
       ▼
     winfm
       │
   ┌───┴─────────────────┐
   │                     │
   ▼                     ▼
System / Battery   Native Interfaces
                       │
                 ┌─────┴─────┐
                 ▼           ▼
             Clipboard     Volume
                 │           │
                 └─────┬─────┘
                       ▼
                 Windows APIs
```

This keeps application code simple while allowing `winfm` to expose native Windows functionality.

## Repository

[github.com/shoya-129/winfm](https://github.com/shoya-129/winfm)
