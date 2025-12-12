# Quickstart Guide: Snapcast Controller Application

**Date**: 2025-12-12
**Feature**: 001-snapcast-controller

## Overview

This guide explains how to set up and use the Snapcast Controller application to control a single room's audio playback using a physical hardware controller.

## Prerequisites

- Snapcast server running on your network (version 0.27+ recommended)
- USB HID hardware controller (3 knobs, 6 buttons with screens, 3 page buttons)
- Linux computer with USB port (primary platform)
- Network connectivity to Snapcast server

## Installation

**Note**: This section will be updated during implementation. For now, assume a compiled Rust binary.

```bash
# Future: Installation instructions
cargo install snapcast-controller

# Or download pre-built binary
# wget https://github.com/.../snapcast-controller
# chmod +x snapcast-controller
```

## Initial Configuration

### 1. First Run

On first run, the application will create a default configuration file:

```bash
snapcast-controller
```

Output:
```
Configuration file not found. Creating default at:
  ~/.config/snapcast-controller/config.toml

Please edit the configuration file and restart.
```

### 2. Edit Configuration

Open the configuration file in your text editor:

```bash
vi ~/.config/snapcast-controller/config.toml
```

Example configuration:

```toml
[server]
address = "192.168.1.100"  # Replace with your Snapcast server IP
port = 1705                 # Default Snapcast JSON-RPC port

[room]
client_id = "living-room"   # Replace with your room's Snapcast client ID
```

**Finding your client ID**:
1. Run `snapcast-controller --list-clients` (future feature)
2. Or check your Snapcast server configuration
3. Or use the Snapcast mobile app to see client names/IDs

### 3. Connect Hardware Controller

1. Plug the USB HID controller into a USB port
2. Ensure your user has permissions to access USB HID devices

**Linux USB Permissions** (if needed):
```bash
# Add udev rule for USB HID access
sudo nano /etc/udev/rules.d/99-ajazz-controller.rules
```

Add:
```
SUBSYSTEM=="usb", ATTRS{idVendor}=="XXXX", ATTRS{idProduct}=="YYYY", MODE="0666"
```

Replace `XXXX` and `YYYY` with your device's vendor/product IDs (find with `lsusb`).

Reload rules:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### 4. Start Application

```bash
snapcast-controller
```

Expected output:
```
[INFO] Loading configuration from ~/.config/snapcast-controller/config.toml
[INFO] Waiting for hardware controller...
[INFO] Hardware controller connected
[INFO] Connecting to Snapcast server at 192.168.1.100:1705...
[INFO] Connected to Snapcast server (version 0.27.0)
[INFO] Monitoring room: Living Room (living-room)
[INFO] Ready - use hardware controls to manage audio playback
```

The hardware controller screens should now display the room's status.

## Using the Hardware Controller

### Hardware Layout

```
┌─────────────────────────────────────────────┐
│  Hardware Controller                         │
├─────────────────────────────────────────────┤
│                                              │
│   ┌───┐     ┌───┐     ┌───┐                │
│   │ 0 │     │ 1 │     │ 2 │     Knobs       │
│   └───┘     └───┘     └───┘                │
│                                              │
│  ┌─────┐  ┌─────┐  ┌─────┐                 │
│  │  0  │  │  1  │  │  2  │                 │
│  │     │  │     │  │     │  Buttons        │
│  └─────┘  └─────┘  └─────┘  (with screens) │
│                                              │
│  ┌─────┐  ┌─────┐  ┌─────┐                 │
│  │  3  │  │  4  │  │  5  │                 │
│  │     │  │     │  │     │                 │
│  └─────┘  └─────┘  └─────┘                 │
│                                              │
│   [◄]      [▼]      [►]      Page Buttons   │
│                                              │
└─────────────────────────────────────────────┘
```

### Control Mapping

#### Knob 0 (Left): Volume Control
- **Rotate Clockwise**: Increase volume
- **Rotate Counter-Clockwise**: Decrease volume
- Volume adjusts in increments of 5%
- Range: 0% to 100%

#### Knob 1 (Middle): Reserved (Future Use)
- Currently unused
- Future: Per-stream equalizer control

#### Knob 2 (Right): Reserved (Future Use)
- Currently unused
- Future: Latency adjustment or balance control

#### Buttons (Status Page)
When on the Status page, buttons display and control:

- **Button 0**: Mute/Unmute
  - Screen shows: "🔇 MUTE" or "🔊 UNMUTE"
  - Press to toggle mute status

- **Button 1**: Current Stream
  - Screen shows: Current stream name (e.g., "Spotify")
  - Press to enter Stream Selection page

- **Button 2**: Volume Display
  - Screen shows: Current volume percentage (e.g., "Vol: 75%")
  - Press: No action (display only)

- **Button 3**: Connection Status
  - Screen shows: "Connected" or "Disconnected"
  - Press: Attempt reconnect if disconnected

- **Button 4**: Server Info
  - Screen shows: Server hostname or IP
  - Press: No action (display only)

- **Button 5**: Room Name
  - Screen shows: Room name (e.g., "Living Room")
  - Press: No action (display only)

#### Buttons (Stream Selection Page)
When on the Stream Selection page, buttons map to available streams:

- **Buttons 0-5**: Stream selection
  - Each button screen shows a stream name (e.g., "Spotify", "Radio", "AUX")
  - Press to assign room to that stream
  - Returns to Status page after selection

- If more than 6 streams exist, use Page buttons to navigate

#### Page Buttons

- **Page Button 0 (◄)**: Previous page
  - In Stream Selection: Show previous 6 streams
  - In Status: No action

- **Page Button 1 (▼)**: Cycle pages
  - Status → Stream Selection → Settings → Status (loop)

- **Page Button 2 (►)**: Next page
  - In Stream Selection: Show next 6 streams
  - In Status: No action

## Common Tasks

### Adjusting Volume

1. Ensure hardware controller is on Status page
2. Rotate Knob 0 (left knob):
   - Clockwise = increase volume
   - Counter-clockwise = decrease volume
3. Volume changes are sent to server immediately
4. Screen updates to show new volume within 500ms

### Muting Audio

1. On Status page, press Button 0 (Mute/Unmute)
2. Screen changes from "🔊 UNMUTE" to "🔇 MUTE"
3. Audio stops immediately
4. Press again to unmute

### Changing Audio Stream

1. Press Page Button 1 to switch to Stream Selection page
2. Button screens now show available streams:
   - Button 0: "Spotify"
   - Button 1: "Radio"
   - Button 2: "AUX"
   - Button 3: "Airplay"
   - Button 4: "Bluetooth"
   - Button 5: "Line In"
3. Press the button for desired stream
4. Room switches to new stream
5. Automatically returns to Status page
6. Current stream shown on Button 1 screen

### Handling Connection Loss

**Server Connection Lost**:
1. All button screens show "Server Disconnected"
2. Application automatically attempts to reconnect
3. Reconnection status shown on screens:
   - "Reconnecting... (1s)"
   - "Reconnecting... (2s)"
   - etc.
4. When reconnected, status is restored

**Hardware Controller Disconnected**:
1. If USB cable is unplugged, application logs disconnection
2. Application waits for device to be reconnected
3. When replugged, screens restore previous state

## Troubleshooting

### Hardware Controller Not Detected

**Problem**: Application says "Waiting for hardware controller..." indefinitely

**Solutions**:
1. Check USB cable connection
2. Try different USB port
3. Check USB permissions (see Initial Configuration section)
4. Verify device with `lsusb` (should show Ajazz device)
5. Check dmesg for USB errors: `dmesg | tail`

### Cannot Connect to Snapcast Server

**Problem**: "Connection failed" error on screens

**Solutions**:
1. Verify server address in config: `cat ~/.config/snapcast-controller/config.toml`
2. Ping server: `ping <server_address>`
3. Check Snapcast server is running: `systemctl status snapserver`
4. Verify port 1705 is open: `nc -zv <server_address> 1705`
5. Check firewall rules on server

### Room Not Found

**Problem**: "Invalid client ID" error

**Solutions**:
1. Verify client_id in config matches actual Snapcast client ID
2. List available clients (future feature): `snapcast-controller --list-clients`
3. Check Snapcast server logs for client name
4. Ensure Snapcast client (snapclient) is running on the room's device

### Screen Updates Delayed

**Problem**: Volume changes don't show on screen immediately

**Expected Behavior**:
- Screen should update within 500ms for local actions
- Screen should update within 2 seconds for server notifications

**Solutions**:
1. Check network latency to server: `ping <server_address>`
2. Restart application
3. Check CPU usage (may be system resource issue)

### Volume Control Not Working

**Problem**: Rotating knob doesn't change volume

**Solutions**:
1. Check if room is connected (Button 3 should show "Connected")
2. Check if volume is showing on screen (Button 2)
3. Restart application
4. Check Snapcast server logs for errors
5. Test volume control from Snapcast mobile app (verify server accepts commands)

## Configuration Reference

### config.toml Format

```toml
[server]
# Required: Snapcast server IP address or hostname
address = "192.168.1.100"

# Optional: Snapcast JSON-RPC port (default: 1705)
port = 1705

[room]
# Required: Snapcast client ID for this controller
# Must match the ID from your Snapcast server configuration
client_id = "living-room"

# Optional: Future authentication
# [server.auth]
# username = "admin"
# password = "secret"
```

### Configuration File Locations

**Linux**: `~/.config/snapcast-controller/config.toml`
**macOS** (future): `~/Library/Application Support/snapcast-controller/config.toml`
**Windows** (future): `%APPDATA%\snapcast-controller\config.toml`

## Advanced Usage

### Running as System Service (Linux)

Create systemd service file: `/etc/systemd/system/snapcast-controller.service`

```ini
[Unit]
Description=Snapcast Hardware Controller
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
ExecStart=/usr/local/bin/snapcast-controller
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable snapcast-controller
sudo systemctl start snapcast-controller
```

Check status:
```bash
sudo systemctl status snapcast-controller
journalctl -u snapcast-controller -f
```

### Multiple Controllers

To run multiple controllers for different rooms:

1. Create separate configuration files:
   - `~/.config/snapcast-controller/living-room.toml`
   - `~/.config/snapcast-controller/bedroom.toml`

2. Run with explicit config paths:
   ```bash
   snapcast-controller --config ~/.config/snapcast-controller/living-room.toml
   snapcast-controller --config ~/.config/snapcast-controller/bedroom.toml
   ```

**Note**: Each instance requires a separate hardware controller connected via USB.

## Next Steps

After successfully setting up the controller:

1. Test all three user stories:
   - ✅ P1: Verify connection and room status display
   - ✅ P2: Confirm real-time status updates
   - ✅ P3: Test volume, mute, and stream controls

2. Configure as systemd service for automatic startup

3. Deploy additional controllers to other rooms

## Support and Resources

- **Snapcast Documentation**: https://github.com/badaix/snapcast
- **Application Logs**: Check stdout/stderr or journal if running as service
- **Report Issues**: (Future: GitHub issues URL)

## Appendix: Hardware Button Screen Layouts

### Status Page Layout

```
┌─────┬─────┬─────┐
│ 🔊  │ Now │Vol: │
│UNMUT│Spot │ 75% │
│     │ ify │     │
└─────┴─────┴─────┘
┌─────┬─────┬─────┐
│Conn │Servr│Room │
│ected│.100 │Live │
│  ✓  │:1705│ Rm  │
└─────┴─────┴─────┘
```

### Stream Selection Page Layout

```
┌─────┬─────┬─────┐
│Spot │Radio│ AUX │
│ ify │     │Input│
│  ▶  │  ⏸  │  ⏸  │
└─────┴─────┴─────┘
┌─────┬─────┬─────┐
│Airpl│Bluet│Line │
│ ay  │ooth │  In │
│  ⏸  │  ⏸  │  ⏸  │
└─────┴─────┴─────┘
```

Symbol meanings:
- ▶ = Currently playing
- ⏸ = Available (not playing)
- ✓ = Connected
- 🔊 = Unmuted
- 🔇 = Muted
