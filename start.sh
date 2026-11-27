#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bg_args=()
gtk_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    # Boolean switches for background process (no value -> shift 1)
    --build-rust|--build-vue)
      bg_args+=("$1")
      shift
      ;;

    # Flags with values for webgtk (flag + value -> shift 2)
    -u|--url|--kiosk-url|-m|--monitor|--target-monitor|-t|--title)
      if [[ -z "${2:-}" ]]; then
        echo "Error: $1 requires a value" >&2
        exit 1
      fi
      gtk_args+=("$1" "$2")
      shift 2
      ;;

    # Equals-style flags for webgtk (--flag=value -> shift 1)
    --url=*|--kiosk-url=*|--monitor=*|--target-monitor=*|--title=*)
      gtk_args+=("$1")
      shift
      ;;
  esac
done

bash "$SCRIPT_DIR/start_background_process.sh" "${bg_args[@]}"
bash "$SCRIPT_DIR/start_webgtk.sh" "${gtk_args[@]}"
