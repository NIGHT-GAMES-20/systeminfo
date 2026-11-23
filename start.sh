#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "$SCRIPT_DIR/start_background_process.sh"
bash "$SCRIPT_DIR/start_webgtk.sh" "${1:-}" "${2:-}"
