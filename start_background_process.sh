#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_DIR="$PROJECT_DIR/release"
INSTALL_DIR="/opt/systeminfo"
SERVICE_NAME="systeminfo"
CADDY_CONF="/etc/caddy/conf.d/systeminfo.caddy"
HOSTNAME_LOCAL="$(hostname).local"

SKIP_RUST_BUILD=1
SKIP_VUE_BUILD=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build-rust)
      SKIP_RUST_BUILD=""
      shift # Move past the flag
      ;;
    --build-vue)
      SKIP_VUE_BUILD=""
      shift # Move past the flag
      ;;
    -h|--help)
      echo "Usage: $0 [--build-rust] [--build-vue]"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

if [[ -z "$SKIP_VUE_BUILD" ]]; then
  echo "==> [1/5] Building Vue frontend..."
  (cd "$PROJECT_DIR/frontend" && npm install && npm run build)
else
  echo "==> [1/5] Skipping Vue frontend!!!"
fi

if [[ -z "$SKIP_RUST_BUILD" ]]; then
  echo "==> [2/5] Building Rust relay (release)..."
  (cd "$PROJECT_DIR/relay" && cargo build --release)
else
  echo "==> [1/5] Skipping Rust relay (release)!!!"
fi

echo "==> [3/5] Installing binaries and static files to $INSTALL_DIR..."
sudo mkdir -p "$INSTALL_DIR/www"
sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true

if [[ -n "$SKIP_RUST_BUILD" ]]; then
  if [[ -e "$INSTALL_DIR/relay" ]]; then
    echo "==> Relay binary found. Skipping Installing Relay Binary"
  else
    echo "==> Relay binary Not Found. Checking $RELEASE_DIR for Binary"
    if [[ -f "$RELEASE_DIR/relay" ]]; then
      echo "==> Installing Relay Binary from $RELEASE_DIR"
      sudo cp "$RELEASE_DIR/relay" "$INSTALL_DIR/relay"
      sudo chmod +x "$INSTALL_DIR/relay"
    else
      echo "Error: $RELEASE_DIR doesnt Exist. Failed to Find Valid Binary for Background Relay" >&2
      exit 1
    fi
  fi
else
  sudo cp "$PROJECT_DIR/relay/target/release/relay" "$INSTALL_DIR/relay"
  sudo chmod +x "$INSTALL_DIR/relay"
fi

REQUIRED_FILES=("index.html" "assets/index-*.css" "assets/index-*.js")
MISSING=""
if [[ -n "$SKIP_VUE_BUILD" ]]; then
  for pattern in "${REQUIRED_FILES[@]}"; do
    matches=("$INSTALL_DIR"/"www"/$pattern)
    if [[ ${#matches[@]} -eq 0 || ! -f "${matches[0]}" ]]; then
      MISSING=1
      echo "==> Frontend file matching '$pattern' in '$INSTALL_DIR'/www missing."
    fi
  done
  if [[ -n "$MISSING" ]]; then
    if [[ -d "$RELEASE_DIR/frontend" ]]; then
      echo "==> Installing Frontend Files from $RELEASE_DIR"
      sudo rm -rf "$INSTALL_DIR/www/"*
      sudo cp -r "$RELEASE_DIR/frontend/"* "$INSTALL_DIR/www/"
    else
      echo "Error: $RELEASE_DIR doesnt Exist. Failed to Find Valid Frontend Files" >&2
      exit 1
    fi
  fi
else
  sudo rm -rf "$INSTALL_DIR/www/"*
  sudo cp -r "$PROJECT_DIR/frontend/dist/"* "$INSTALL_DIR/www/"
fi

PACKAGE="caddy"

if dpkg-query -W -f='${Status}' "$PACKAGE" 2>/dev/null | grep -q "install ok installed"; then
    echo "$PACKAGE is already installed."
else
    echo "$PACKAGE is not installed. Installing now..."
    sudo apt-get update && sudo apt-get install -y "$PACKAGE"
fi

echo "==> [4/5] Ensuring systemd service & global Caddy config exist..."

sudo systemctl start caddy

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
  sudo tee /etc/caddy/Caddyfile > /dev/null << 'MAIN_CADDY_EOF'
{
    auto_https off
}
:80 {
    import /etc/caddy/conf.d/*.caddy
}
MAIN_CADDY_EOF
fi

sudo tee "$CADDY_CONF" > /dev/null << CADDY_EOF
redir /systeminfo /systeminfo/ permanent

handle /systeminfo/api/* {
    reverse_proxy 127.0.0.1:9870
}

handle_path /systeminfo/* {
    root * ${INSTALL_DIR}/www
    file_server
    try_files {path} /index.html
}
CADDY_EOF

sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy


echo "==> [5/5] Starting $SERVICE_NAME..."
sudo systemctl start "$SERVICE_NAME"

echo "Done! Live at http://localhost/systeminfo/"
echo "If Avahi Is Enabled then also available at http://${HOSTNAME_LOCAL}/systeminfo/"
