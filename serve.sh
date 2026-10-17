#!/usr/bin/env bash
set -euo pipefail

# Move to the script's directory (repo root)
cd "$(dirname "$0")"

PORT="${1:-8080}"

if ! command -v python3 &>/dev/null; then
    echo "❌ Error: python3 is required to run the local server."
    exit 1
fi

echo ""
echo "⚽ Vibe PES 2019 Server"
echo "🌐 Running at: http://localhost:${PORT}"
echo "⏹️  Press Ctrl+C to stop the server"
echo ""

exec python3 -m http.server "$PORT" --directory web
