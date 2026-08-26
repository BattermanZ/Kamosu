#!/usr/bin/env bash
# PROTOTYPE — serves the #36 visual-identity mockups over the LAN. Port 8765 is UFW-allowed.
cd "$(dirname "$0")"
exec python3 -m http.server 8765 --bind 0.0.0.0
