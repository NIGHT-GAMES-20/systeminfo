#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="/opt/systeminfo"
SERVICE_NAME="systeminfo"
CADDY_CONF="/etc/caddy/conf.d/systeminfo.caddy"
HOSTNAME_LOCAL="$(hostname).local"

echo "==> [1/5] Building Vue frontend..."
(cd "$PROJECT_DIR/frontend" && npm install && npm run build)

echo "==> [2/5] Building Rust relay (release)..."
(cd "$PROJECT_DIR/relay" && cargo build --release)

echo "==> [3/5] Installing binaries and static files to $INSTALL_DIR..."
sudo mkdir -p "$INSTALL_DIR/www"
sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true

sudo cp "$PROJECT_DIR/relay/target/release/relay" "$INSTALL_DIR/relay"
sudo chmod +x "$INSTALL_DIR/relay"

sudo rm -rf "$INSTALL_DIR/www/"*
sudo cp -r "$PROJECT_DIR/frontend/dist/"* "$INSTALL_DIR/www/"

echo "==> [4/5] Ensuring systemd service & global Caddy config exist..."
if [ ! -f "/etc/systemd/system/${SERVICE_NAME}.service" ]; then
    sudo tee "/etc/systemd/system/${SERVICE_NAME}.service" > /dev/null << SERVICE_EOF
[Unit]
Description=SystemInfo Hardware Telemetry Relay
After=network.target

[Service]
Type=simple
User=root
ExecStart=${INSTALL_DIR}/relay
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
SERVICE_EOF
    sudo systemctl daemon-reload
    sudo systemctl enable "$SERVICE_NAME"
fi

  sudo mkdir -p /etc/caddy/conf.d
  if ! grep -q "import /etc/caddy/conf.d/\*.caddy" /etc/caddy/Caddyfile 2>/dev/null; then
      echo "import /etc/caddy/conf.d/*.caddy" | sudo tee /etc/caddy/Caddyfile > /dev/null
  fi

  sudo tee "$CADDY_CONF" > /dev/null << CADDY_EOF
:80 {
  redir /systeminfo /systeminfo/ permanent

  handle /systeminfo/api/* {
      reverse_proxy 127.0.0.1:9870
  }
  handle_path /systeminfo/*  {
      root * ${INSTALL_DIR}/www
      file_server
      try_files {path} /index.html
  }
}
CADDY_EOF
  sudo systemctl reload caddy


echo "==> [5/5] Starting $SERVICE_NAME..."
sudo systemctl start "$SERVICE_NAME"

echo "Done! Live at http://localhost/systeminfo/"
echo "If Avahi Is Enabled then also available at http://${HOSTNAME_LOCAL}/systeminfo/"
