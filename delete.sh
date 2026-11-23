#!/usr/bin/env bash
set -euo pipefail

echo "==> [1/4] Closing running kiosk browser & removing autostarts..."
pkill -f "systeminfo-kiosk-viewer.py" 2>/dev/null || true
rm -f "$HOME/.local/bin/systeminfo-kiosk-viewer.py"
rm -f "$HOME/.config/autostart/systeminfo-kiosk.desktop"
rm -f "$HOME/.config/autostart/kiosk.desktop"
rm -f "$HOME/.config/systeminfo-kiosk.env"
rm -f "$HOME/.local/bin/start-systeminfo-kiosk.sh"
rm -f "$HOME/.local/bin/kill-systeminfo-kiosk.sh"
rm -rf "$HOME/.local/share/systeminfo-kiosk" "$HOME/.cache/systeminfo-kiosk"

echo "==> [2/4] Stopping and removing systemd service..."
for svc in systeminfo systeminfo-relay; do
    if systemctl list-unit-files | grep -q "^${svc}.service"; then
        sudo systemctl stop "$svc" 2>/dev/null || true
        sudo systemctl disable "$svc" 2>/dev/null || true
    fi
    sudo rm -f "/etc/systemd/system/${svc}.service"
done
sudo systemctl daemon-reload

echo "==> [3/4] Removing /opt/systeminfo..."
sudo rm -rf /opt/systeminfo

echo "==> [4/4] Removing Caddy site config..."
sudo rm -f /etc/caddy/conf.d/systeminfo.caddy
if systemctl is-active --quiet caddy; then
    sudo systemctl reload caddy 2>/dev/null || true
fi

echo ""
echo "Cleanup complete!"
