#!/usr/bin/env bash
set -euo pipefail

DEFAULT_URL="http://localhost/systeminfo/"
VIEWER_SCRIPT="$HOME/.local/bin/systeminfo-kiosk-viewer.py"
LAUNCHER="$HOME/.local/bin/start-systeminfo-kiosk.sh"
KILLER="$HOME/.local/bin/kill-systeminfo-kiosk.sh"
AUTOSTART_FILE="$HOME/.config/autostart/systeminfo-kiosk.desktop"
CONF_FILE="$HOME/.config/systeminfo-kiosk.env"

# Optional CLI arg: ./start_webgtk.sh [url] [monitor_index_or_name] (e.g., 0, 1, HDMI-0, HDMI-1)
# No arg = auto: default URL and first non-primary monitor if one exists, otherwise the primary.
KIOSK_URL="${1:-${KIOSK_URL:-$DEFAULT_URL}}"
TARGET_MONITOR="${2:-${MONITOR:-}}"



echo "==> [1/4] Checking Python WebKitGTK dependencies..."
if ! python3 -c "import gi; gi.require_version('Gtk', '3.0'); gi.require_version('WebKit2', '4.1')" 2>/dev/null && \
   ! python3 -c "import gi; gi.require_version('Gtk', '3.0'); gi.require_version('WebKit2', '4.0')" 2>/dev/null; then
    echo "Installing python3-gi and WebKit2GTK..."
    sudo apt-get update -qq
    sudo apt-get install -y python3-gi gir1.2-gtk-3.0 gir1.2-webkit2-4.1
fi

echo "==> [2/4] Creating WebKitGTK viewer ($VIEWER_SCRIPT)..."
mkdir -p "$HOME/.local/bin" "$HOME/.config"
cat << 'PYTHON_EOF' > "$VIEWER_SCRIPT"
#!/usr/bin/env python3
import os
import sys
import signal
import argparse
from urllib.parse import urlparse
import gi

gi.require_version("Gtk", "3.0")
for ver in ("4.1", "4.0"):
    try:
        gi.require_version("WebKit2", ver)
        break
    except ValueError:
        continue

from gi.repository import Gtk, WebKit2, Gdk, GLib


# Caps requestAnimationFrame to __FPS__ fps. Most dashboards/charts animate via rAF, and
# every frame is a style+layout+paint+composite pass, so this is usually the biggest CPU win.
RAF_THROTTLE_JS = r"""
(function () {
  if (window.__rafThrottle) return;
  window.__rafThrottle = true;
  var gap = 1000 / __FPS__;
  var native = window.requestAnimationFrame.bind(window);
  var queue = [], running = [], nextId = 1, armed = false, last = -1e9;

  function arm() {
    if (armed) return;
    armed = true;
    var wait = last + gap - performance.now() - 8;
    if (wait > 0) setTimeout(function () { native(tick); }, wait);
    else native(tick);
  }
  function tick(t) {
    armed = false;
    if (t - last < gap - 4) { if (queue.length) arm(); return; }
    last = t;
    running = queue; queue = [];
    for (var i = 0; i < running.length; i++) {
      var e = running[i];
      if (e.dead) continue;
      try { e.cb(t); } catch (err) { setTimeout(function () { throw err; }); }
    }
    running = [];
    if (queue.length) arm();
  }
  function kill(list, id) {
    for (var i = 0; i < list.length; i++) if (list[i].id === id) list[i].dead = true;
  }
  window.requestAnimationFrame = function (cb) {
    var id = nextId++; queue.push({ id: id, cb: cb }); arm(); return id;
  };
  window.cancelAnimationFrame = function (id) { kill(queue, id); kill(running, id); };
})();
"""

NO_ANIM_CSS = "*,*::before,*::after{animation:none!important;transition:none!important;scroll-behavior:auto!important}"


def env_int(name, default=0):
    try:
        return int(os.environ.get(name) or default)
    except ValueError:
        return default


def env_flag(name):
    return os.environ.get(name, "0").strip().lower() in ("1", "true", "yes", "on")


def log(msg):
    # Goes to /tmp/systeminfo-kiosk.log (the launcher redirects stderr there)
    print(f"[kiosk] {msg}", file=sys.stderr, flush=True)


def describe_monitors(display):
    lines = []
    for i in range(display.get_n_monitors()):
        m = display.get_monitor(i)
        g = m.get_geometry()
        lines.append(f"  [{i}] model={m.get_model()!r} primary={m.is_primary()} {g.width}x{g.height}+{g.x}+{g.y}")
    return "\n".join(lines)


def try_set(settings, name, value):
    # Setters differ between WebKitGTK versions; skip any that don't exist.
    setter = getattr(settings, "set_" + name, None)
    if setter is None:
        return
    try:
        setter(value)
    except Exception:
        pass


def resolve_monitor(display, spec):
    n_monitors = display.get_n_monitors()
    if n_monitors == 0:
        return 0

    if spec is not None and str(spec).strip() != "":
        spec_str = str(spec).strip()
        # Match by numeric index (0, 1, ...)
        if spec_str.isdigit():
            return min(int(spec_str), n_monitors - 1)
        # Match by connector/model name (e.g., HDMI-0, HDMI-1)
        for i in range(n_monitors):
            model = display.get_monitor(i).get_model() or ""
            if model.lower() == spec_str.lower():
                return i
        log(f"monitor {spec_str!r} not found; falling back to auto")

    # Auto (default): first non-primary monitor if available, otherwise the primary one
    for i in range(n_monitors):
        if not display.get_monitor(i).is_primary():
            return i
    for i in range(n_monitors):
        if display.get_monitor(i).is_primary():
            return i
    return 0


def build_content_manager(fps_cap, no_anim):
    ucm = WebKit2.UserContentManager()
    frames = WebKit2.UserContentInjectedFrames.ALL_FRAMES
    if fps_cap > 0:
        src = RAF_THROTTLE_JS.replace("__FPS__", str(fps_cap))
        ucm.add_script(WebKit2.UserScript.new(
            src, frames, WebKit2.UserScriptInjectionTime.START, None, None))
    if no_anim:
        ucm.add_style_sheet(WebKit2.UserStyleSheet.new(
            NO_ANIM_CSS, frames, WebKit2.UserStyleLevel.USER, None, None))
    return ucm


def main():
    parser = argparse.ArgumentParser(description="SystemInfo WebKitGTK Kiosk Viewer")
    parser.add_argument("url", nargs="?", default="http://127.0.0.1/systeminfo/")
    parser.add_argument("-m", "--monitor", default=os.environ.get("MONITOR", ""), help="Monitor index (0, 1) or model (HDMI-0)")
    parser.add_argument("--debug", action="store_true", help="Enable WebKit Inspector")
    args = parser.parse_args()

    debug_mode = args.debug or os.environ.get("DEBUG", "0").lower() in ("1", "true", "yes")

    # Tuning knobs (set in ~/.config/systeminfo-kiosk.env)
    hw_accel = os.environ.get("HW_ACCEL", "ondemand").strip().lower()
    fps_cap = env_int("FPS_CAP", 20)
    no_anim = env_flag("NO_ANIM")
    mem_limit_mb = env_int("MEM_LIMIT_MB", 0)
    reload_minutes = env_int("RELOAD_MINUTES", 0)

    data_dir = os.path.expanduser("~/.local/share/systeminfo-kiosk")
    cache_dir = os.path.expanduser("~/.cache/systeminfo-kiosk")
    os.makedirs(data_dir, exist_ok=True)
    os.makedirs(cache_dir, exist_ok=True)

    # Ask WebKit to trim caches / run GC earlier once the web process nears this size.
    # Must be set before the first web process is spawned. Off unless MEM_LIMIT_MB > 0.
    if mem_limit_mb > 0:
        try:
            mps = WebKit2.MemoryPressureSettings.new()
            mps.set_memory_limit(mem_limit_mb)
            WebKit2.WebsiteDataManager.set_memory_pressure_settings(mps)
        except Exception as exc:
            print(f"MEM_LIMIT_MB ignored (not supported by this WebKitGTK): {exc}", file=sys.stderr)

    data_manager = WebKit2.WebsiteDataManager(
        base_data_directory=data_dir,
        base_cache_directory=cache_dir,
    )
    context = WebKit2.WebContext.new_with_website_data_manager(data_manager)
    # Smallest in-memory resource cache; a single localhost page doesn't need more.
    context.set_cache_model(WebKit2.CacheModel.DOCUMENT_VIEWER)

    display = Gdk.Display.get_default()
    screen = display.get_default_screen()

    win = Gtk.Window(title="SystemInfo Kiosk")
    win.set_decorated(False)
    win.set_skip_taskbar_hint(True)
    win.set_skip_pager_hint(True)

    # Placement is re-evaluated whenever the monitor layout changes (a screen is plugged in,
    # unplugged, or the primary flag changes), so a login-time race or hotplug can't leave the
    # kiosk stuck on the wrong screen. With no monitor pinned, the rule is always:
    # first non-primary monitor if one exists, otherwise the primary.
    placed = {"key": None}

    def place_window():
        if display.get_n_monitors() == 0:
            return False
        idx = resolve_monitor(display, args.monitor)
        geom = display.get_monitor(idx).get_geometry()
        key = (idx, geom.x, geom.y, geom.width, geom.height)
        if key == placed["key"]:
            return False  # nothing changed
        log(f"monitors:\n{describe_monitors(display)}\n[kiosk] using monitor {idx}")
        if placed["key"] is not None and win.get_window() is not None:
            win.unfullscreen()  # needed before re-targeting an already fullscreen window
        placed["key"] = key
        win.set_default_size(geom.width, geom.height)
        win.move(geom.x, geom.y)
        win.fullscreen_on_monitor(screen, idx)
        return False  # also lets this serve as a one-shot GLib timeout

    place_window()

    # Enforce placement again on realize so window managers don't snap to the cursor's screen
    def on_realize(_widget):
        placed["key"] = None
        place_window()

    win.connect("realize", on_realize)

    # randr fires several events per change; wait for them to settle, then re-place
    screen.connect("monitors-changed", lambda *_: GLib.timeout_add(700, place_window))

    ucm = build_content_manager(fps_cap, no_anim)
    webview = WebKit2.WebView(web_context=context, user_content_manager=ucm)
    settings = webview.get_settings()

    # JS -> native bridge so the dashboard can offer an "Exit kiosk" button. It only exists
    # inside this viewer: window.webkit.messageHandlers.kioskQuit.postMessage("quit")
    ucm.register_script_message_handler("kioskQuit")

    def on_kiosk_quit(*_args):
        # Only honour it from our own page, never from whatever a link might navigate to.
        if urlparse(webview.get_uri() or "").netloc == urlparse(args.url).netloc:
            Gtk.main_quit()

    ucm.connect("script-message-received::kioskQuit", on_kiosk_quit)

    policies = {
        "never": WebKit2.HardwareAccelerationPolicy.NEVER,
        "ondemand": WebKit2.HardwareAccelerationPolicy.ON_DEMAND,
        "always": WebKit2.HardwareAccelerationPolicy.ALWAYS,
    }
    settings.set_hardware_acceleration_policy(policies.get(hw_accel, policies["ondemand"]))
    settings.set_enable_html5_local_storage(True)
    settings.set_enable_developer_extras(debug_mode)
    try_set(settings, "enable_page_cache", False)
    try_set(settings, "enable_smooth_scrolling", False)
    try_set(settings, "enable_write_console_messages_to_stdout", False)

    def on_key_press(_widget, event):
        if event.keyval == Gdk.KEY_F5 or (
            (event.state & Gdk.ModifierType.CONTROL_MASK) and event.keyval in (Gdk.KEY_r, Gdk.KEY_R)
        ):
            webview.reload_bypass_cache()
            return True
        if debug_mode and event.keyval == Gdk.KEY_F12:
            webview.get_inspector().show()
            return True
        return False

    win.connect("key-press-event", on_key_press)
    win.connect("destroy", Gtk.main_quit)

    # Optional periodic reload: caps slow memory growth in a page that runs for weeks.
    if reload_minutes > 0:
        def periodic_reload():
            webview.reload_bypass_cache()
            return True
        GLib.timeout_add_seconds(reload_minutes * 60, periodic_reload)

    # Keep retrying until the page loads: covers boot ordering (relay/Caddy still starting)
    # and redeploys that restart the relay, instead of sitting on an error page forever.
    retry_pending = {"on": False}

    def schedule_retry(delay):
        if retry_pending["on"]:
            return
        retry_pending["on"] = True

        def go():
            retry_pending["on"] = False
            webview.load_uri(args.url)
            return False

        GLib.timeout_add_seconds(delay, go)

    def on_load_failed(_view, _event, _uri, error):
        try:
            if error.code == int(WebKit2.NetworkError.CANCELLED):
                return False  # superseded by another load (e.g. a reload); not a real failure
        except Exception:
            pass
        schedule_retry(5)
        return False  # still show WebKit's error page until the retry lands

    webview.connect("load-failed", on_load_failed)
    webview.connect("web-process-terminated", lambda _view, _reason: schedule_retry(2))

    webview.load_uri(args.url)
    win.add(webview)
    win.show_all()

    signal.signal(signal.SIGINT, lambda *_: Gtk.main_quit())
    signal.signal(signal.SIGTERM, lambda *_: Gtk.main_quit())
    Gtk.main()


if __name__ == "__main__":
    main()
PYTHON_EOF
chmod +x "$VIEWER_SCRIPT"

echo "==> [3/4] Creating standalone launcher ($LAUNCHER), tuning config & desktop autostart..."
# Tuning config (sourced by the launcher). Created if missing; on re-runs any knob that is
# not in the file yet is appended, and your existing values are never touched.
# Values after ':-' are defaults; an env var set on the command line wins over this file.
if [ ! -f "$CONF_FILE" ]; then
    echo "# SystemInfo kiosk tuning (sourced by start-systeminfo-kiosk.sh). Restart the kiosk after editing." > "$CONF_FILE"
fi
ensure_conf() { # key default comment
    grep -q "^$1=" "$CONF_FILE" || printf '# %s\n%s=${%s:-%s}\n' "$3" "$1" "$1" "$2" >> "$CONF_FILE"
}
ensure_conf HW_ACCEL       ondemand  "HW_ACCEL: never | ondemand | always (page compositing on CPU / GPU only when needed / always GPU)"
ensure_conf FPS_CAP        20        "FPS_CAP: max requestAnimationFrame rate; 0 = no cap"
ensure_conf NO_ANIM        0         "NO_ANIM: 1 = kill all CSS animations/transitions (also a great 'is it the CSS?' test)"
ensure_conf DISABLE_DMABUF 1         "DISABLE_DMABUF: 1 = WEBKIT_DISABLE_DMABUF_RENDERER=1 (old GPU/driver workaround; try 0)"
ensure_conf MEM_LIMIT_MB   0         "MEM_LIMIT_MB: >0 = ask WebKit to trim memory earlier near this web-process size (e.g. 200)"
ensure_conf RELOAD_MINUTES 0         "RELOAD_MINUTES: >0 = reload the page this often to cap memory creep (e.g. 360)"
ensure_conf SYSTEMINFO_SERVICE systeminfo "SYSTEMINFO_SERVICE: systemd unit the kiosk waits for before starting"
ensure_conf RELAY_PORT     9870      "RELAY_PORT: relay port the kiosk waits to accept connections (matches your Caddy reverse_proxy)"
ensure_conf KIOSK_WAIT_SECS 180      "KIOSK_WAIT_SECS: how long autostart waits for service + relay + Caddy; it starts anyway after this"
ensure_conf MONITOR        ""        "MONITOR: leave empty for auto (first non-primary monitor, else primary); or pin an index like 0 / 1"
ensure_conf KIOSK_URL      "$DEFAULT_URL" "KIOSK_URL: page the kiosk shows (CLI arg 2 overrides this; autostart uses this value)"

cat << 'LAUNCHER_EOF' > "$LAUNCHER"
#!/usr/bin/env bash
CONF_FILE="$HOME/.config/systeminfo-kiosk.env"
if [ -f "$CONF_FILE" ]; then
    set -a
    . "$CONF_FILE"
    set +a
fi

DEFAULT_URL="http://localhost/systeminfo/"
KIOSK_URL="${1:-${KIOSK_URL:-$DEFAULT_URL}}"
CHECK_URL="http://127.0.0.1/systeminfo/"
VIEWER_SCRIPT="$HOME/.local/bin/systeminfo-kiosk-viewer.py"
TARGET_MONITOR="${2:-${MONITOR:-}}"

SERVICE_NAME="${SYSTEMINFO_SERVICE:-systeminfo}"
RELAY_PORT="${RELAY_PORT:-9870}"
WAIT_SECS="${KIOSK_WAIT_SECS:-180}"

# Ready = systemd service active + relay port accepting connections + Caddy serving the page.
# (Caddy alone answers 200 for the static files even when the relay is down, so check all three.)
service_ready() {
    if command -v systemctl >/dev/null 2>&1 && systemctl cat "$SERVICE_NAME" >/dev/null 2>&1; then
        systemctl is-active --quiet "$SERVICE_NAME" || return 1
    fi
    (exec 3<>"/dev/tcp/127.0.0.1/${RELAY_PORT}") 2>/dev/null || return 1
    curl -fsS -m 1 -o /dev/null "$CHECK_URL" 2>/dev/null || return 1
}

ready=0
if [ "$KIOSK_URL" = "$DEFAULT_URL" ]; then
    for ((i = 0; i < WAIT_SECS * 2; i++)); do
        if service_ready; then
            ready=1
            break
        fi
        sleep 0.5
    done
    if [ "$ready" -ne 1 ]; then
        # Not fatal: the viewer keeps retrying the page on its own until the backend is up.
        echo "WARNING: $SERVICE_NAME / :$RELAY_PORT / $CHECK_URL not ready after ${WAIT_SECS}s; starting viewer anyway." >&2
    fi
fi

pkill -f "[p]ython3.*systeminfo-kiosk-viewer.py" 2>/dev/null || true

export DISPLAY="${DISPLAY:-:0}"
if [ "${DISABLE_DMABUF:-1}" = "1" ]; then
    export WEBKIT_DISABLE_DMABUF_RENDERER=1
fi
export DEBUG="${DEBUG:-0}"
export DEBUG_PORT="${DEBUG_PORT:-9222}"

MONITOR_ARGS=()
if [ -n "$TARGET_MONITOR" ]; then
    MONITOR_ARGS=(--monitor "$TARGET_MONITOR")
fi

if [[ "$DEBUG" == "1" || "$DEBUG" == "true" ]]; then
    export WEBKIT_INSPECTOR_SERVER="127.0.0.1:${DEBUG_PORT}"
    nohup python3 "$VIEWER_SCRIPT" "$KIOSK_URL" "${MONITOR_ARGS[@]}" --debug </dev/null >/tmp/systeminfo-kiosk.log 2>&1 &
else
    nohup python3 "$VIEWER_SCRIPT" "$KIOSK_URL" "${MONITOR_ARGS[@]}" </dev/null >/tmp/systeminfo-kiosk.log 2>&1 &
fi
disown
LAUNCHER_EOF
chmod +x "$LAUNCHER"

cat << 'KILLER_EOF' > "$KILLER"
#!/usr/bin/env bash
# Stops the SystemInfo kiosk viewer. Its WebKit child processes exit together with it.
# Autostart is untouched, so the kiosk comes back at the next login / start-systeminfo-kiosk.sh.
PATTERN="[p]ython3.*systeminfo-kiosk-viewer.py"

if ! pgrep -f "$PATTERN" >/dev/null 2>&1; then
    echo "Kiosk viewer is not running."
    exit 0
fi

pkill -TERM -f "$PATTERN" 2>/dev/null || true   # graceful: the viewer handles SIGTERM
for _ in {1..20}; do
    if ! pgrep -f "$PATTERN" >/dev/null 2>&1; then
        echo "Kiosk viewer stopped."
        exit 0
    fi
    sleep 0.25
done

echo "Viewer ignored SIGTERM after 5s; forcing kill..."
pkill -KILL -f "$PATTERN" 2>/dev/null || true
sleep 0.3
if pgrep -f "$PATTERN" >/dev/null 2>&1; then
    echo "ERROR: could not stop the kiosk viewer." >&2
    exit 1
fi
echo "Kiosk viewer killed."
KILLER_EOF
chmod +x "$KILLER"

mkdir -p "$HOME/.config/autostart"
cat << EOF > "$AUTOSTART_FILE"
[Desktop Entry]
Type=Application
Name=SystemInfo Kiosk
Exec=$LAUNCHER
Terminal=false
X-GNOME-Autostart-enabled=true
EOF

echo "==> [4/4] Launching WebKitGTK kiosk in background..."
# Interactive run: don't sit here for the full autostart wait if the service isn't deployed yet.
KIOSK_WAIT_SECS="${KIOSK_WAIT_SECS:-15}" "$LAUNCHER" "${1:-}" "${2:-}"

echo "Done! Kiosk launched (F5/Ctrl+R to reload, Alt+F4 to close)"
echo "Tuning knobs: $CONF_FILE"
echo "Stop it any time with: $KILLER"
