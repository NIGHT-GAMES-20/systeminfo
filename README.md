# SystemInfo

A Package to display Most of the System Informations of your Device and To make them Available in the localnet

## Contents

| Script | Purpose |
| --- | --- |
| [`start_background_process.sh`](#1-background-service-bg-script) | Starts the relay,frontend systemd service and sets up caddy to reverse proxy port `9870` to `80` |
| [`start_webgtk.sh`](#2-kiosk-viewer-start_webgtksh) | Starts up the WebKitGTK Viewer |
| [`start.sh`](#3-start-start-script) | Combines Both the Previous Start Files into one |
| [`delete.sh`](#4-delete-delete-script) | Deletes all the Configs, Systemd services, autostarts and cleans all the files created in teh startup files   |

## Quick start

```bash
# To Startup only the relay server and the Frontend
# Accessable at http://localhost/systeminfo/ or https://<ip>/systeminfo/
# or https://<hostname>.local/systeminfo/ (if mDNS is configured) 
# If sudo is not used, you need to enter sudo password during step 3-4
sudo ./start_background_process.sh

# Starts the WebKitGTK Viewer
./start_webgtk.sh

```
OR 
```bash

# Start Both the server and the Viewer Together
sudo ./start.sh
```

## Requirements

- OS: Linux (Tested On Linux Mint only)
- Tools Needed: npm, Rust/cargo, Caddy, Python + WebKitGTK
- Sudo Needed - **Yes**
- Ports Used - `9870` Reverse Proxied to port 80 using caddy 

---

## 1. Background service (`start_background_process.sh`)

**What it does**

Builds the Frontend and Relay, and copies all the binaries and statics files to `/opt/systeminfo/`, sets up a systemd service for startup and creates the caddy config and starts the service using systemd 

**Usage**

```bash
./start_background_process.sh
```

**Steps it performs**

1. Installs all the requirements and builds the Vue Frontend
2. Builds the Rust Relay Binary
3. Copies the relay binary to `/opt/systeminfo/` and the frontend static files to `/opt/systeminfo/www`
4. Sets up a Systemd Service and enables it
5. Creates a Caddy config to reverse proxy internal `9870` Port to port `80`
6. Starts the Systemd Service and the information collection process

---

## 2. Kiosk viewer (`start_webgtk.sh`)

Starts up the kiosk Viewer to Display the hosted Frontend using python WebKitGTK

**Usage**

```bash
# first argument takes the url at which the kiosk Starts
# default URL is "http://localhost/systeminfo/

# second argument takes the monitor where the kiosk will be displayed
# default Monitor is the First Non primary Display if Available otherwise the primary display

./start_webgtk.sh [url] [monitor_index_or_name]

```

**Installs / creates**

- Sets up 5 Scripts and helpers
- `systeminfo-kiosk-viewer.py` Contains the main python webkit scripts
- `start-systeminfo-kiosk.sh` used to start up the kiosk window
- `kill-systeminfo-kiosk.sh` kills all the kiosk windows
- `systeminfo-kiosk.desktop` sets up autostart for the kiosk window
- `systeminfo-kiosk.env` config files for the kiosk

**Options / config**

| Setting | Default | Meaning |
| --- | --- | --- |
| HW_ACCEL | ondemand | never / ondemand / always (page compositing on CPU / GPU only when needed / always GPU) |
| FPS_CAP | 20 | max requestAnimationFrame rate; 0 = no cap |
| NO_ANIM | 0 | NO_ANIM: 1 = kill all CSS animations/transitions (also a great 'is it the CSS?' test) |
| DISABLE_DMABUF | 1 | DISABLE_DMABUF: 1 = WEBKIT_DISABLE_DMABUF_RENDERER=1 (old GPU/driver workaround; try 0) |
| MEM_LIMIT_MB | 0 | >0 = ask WebKit to trim memory earlier near this web-process size (e.g. 200) |
| RELOAD_MINUTES | 0 | >0 = reload the page this often to cap memory creep (e.g. 360) |
| SYSTEMINFO_SERVICE | systeminfo | systemd unit the kiosk waits for before starting |
| RELAY_PORT | 9870 | elay port the kiosk waits to accept connections (matches Caddy reverse_proxy) |
| KIOSK_WAIT_SECS | 180 | how long autostart waits for service + relay + Caddy; it starts anyway after this |
| MONITOR | "" | leave empty for auto (first non-primary monitor, else primary); or pin an index like 0 / 1 |
| KIOSK_URL | http://localhost/systeminfo/ | page the kiosk shows (CLI arg 2 overrides this; autostart uses this value) |

**Notes**

- `./start_webgtk.sh [monitor_index_or_name]` (e.g., 0, 1, HDMI-0, HDMI-1) to start the kiosk in a perticular monitor.. defaults to the first non primary monitor
- use `DEBUG=1 DEBUG_PORT=<port> ./start_webgtk.sh ` to startup the devtools on the kiosk (default port `9222`)

---

## 3. Start (`start.sh`)

**What it does**

Combines the Above both scripts into a single runner file

**Usage**

```bash
# Same Argumets as start_webgtk.sh

./start.sh [url] [monitor_index_or_name]
```

---

## 4. Delete (`delete.sh`)

**What it does**

Deletes All of the Scripts Files, Binaries etc which have been copied to `/.local/`, `/opt/systeminfo` , systemd files and caddy config

**Usage**

```bash
./delete.sh
```

**Removes**

- `$HOME/.local/bin/systeminfo-kiosk-viewer.py`
- `$HOME/.local/bin/start-systeminfo-kiosk.sh`
- `$HOME/.local/bin/kill-systeminfo-kiosk.sh`
- `$HOME/.config/autostart/systeminfo-kiosk.desktop`
- `$HOME/.config/autostart/kiosk.desktop`
- `$HOME/.cache/systeminfo-kiosk`
- `$HOME/.config/systeminfo-kiosk.env`
- `/etc/caddy/conf.d/systeminfo.caddy`
- `/opt/systeminfo`
- `/etc/systemd/system/systeminfo.service`
