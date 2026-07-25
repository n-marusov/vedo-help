#!/usr/bin/env bash
# =============================================================================
# Install backup systemd service and timer for VEDO hub
# =============================================================================
# Installs a systemd service and timer that runs the backup script daily at
# 03:00, rotating backups older than 30 days.
#
# Usage:
#   sudo ./scripts/install-backup-timer.sh [--prod] [--user <user>] [--path <project-dir>]
#
# Options:
#   --prod, -p      Use production compose files (default: development)
#   --user <user>   System user to run the backup as (default: root)
#   --path <dir>    Project directory path (default: auto-detected)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default values
BACKUP_USER="root"
BACKUP_FLAGS=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prod|-p)
            BACKUP_FLAGS="--prod"
            shift
            ;;
        --user)
            if [[ $# -lt 2 ]]; then
                echo "[ERROR] --user requires an argument" >&2
                exit 1
            fi
            BACKUP_USER="$2"
            shift 2
            ;;
        --path)
            if [[ $# -lt 2 ]]; then
                echo "[ERROR] --path requires an argument" >&2
                exit 1
            fi
            PROJECT_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: sudo $0 [--prod] [--user <user>] [--path <project-dir>]"
            exit 0
            ;;
        *)
            echo "[ERROR] Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Validate running as root
if [[ $EUID -ne 0 ]]; then
    echo "[ERROR] This script must be run as root (sudo)." >&2
    exit 1
fi

# Validate project directory
if [[ ! -f "$PROJECT_DIR/scripts/backup.sh" ]]; then
    echo "[ERROR] Project directory not found or missing scripts/backup.sh: $PROJECT_DIR" >&2
    exit 1
fi

SERVICE_FILE="/etc/systemd/system/vedo-backup.service"
TIMER_FILE="/etc/systemd/system/vedo-backup.timer"

echo "[INFO] Installing VEDO hub backup systemd service..."

# Create service file
cat > "$SERVICE_FILE" << SERVICEEOF
[Unit]
Description=VEDO hub daily backup
After=network-online.target docker.service
Wants=network-online.target docker.service

[Service]
Type=oneshot
User=$BACKUP_USER
WorkingDirectory=$PROJECT_DIR
ExecStart=/usr/bin/env bash $PROJECT_DIR/scripts/backup.sh $BACKUP_FLAGS
StandardOutput=append:/var/log/vedo-backup.log
StandardError=append:/var/log/vedo-backup.log
Nice=19
IOSchedulingClass=idle

[Install]
WantedBy=multi-user.target
SERVICEEOF

echo "[INFO] Service file created: $SERVICE_FILE"

# Create timer file
cat > "$TIMER_FILE" << TIMEREOF
[Unit]
Description=Daily VEDO hub backup timer
Documentation=https://github.com/vedo-hub/vedo-rag-assistant

[Timer]
OnCalendar=daily
Persistent=true
RandomizedDelaySec=900

[Install]
WantedBy=timers.target
TIMEREOF

echo "[INFO] Timer file created: $TIMER_FILE"

# Reload systemd
systemctl daemon-reload

# Enable and start the timer
systemctl enable vedo-backup.timer
systemctl start vedo-backup.timer

echo ""
echo "[INFO] Backup timer installed and enabled."
echo ""
echo "  Service: vedo-backup.service"
echo "  Timer:   vedo-backup.timer"
echo "  Schedule: Daily at 03:00 (with random delay)"
echo "  Log:     /var/log/vedo-backup.log"
echo ""
echo "  Verify:  systemctl list-timers --all | grep vedo"
echo "  Status:  systemctl status vedo-backup.service"
echo "  Logs:    journalctl -u vedo-backup.service"
echo "  Stop:    sudo systemctl stop vedo-backup.timer"
echo "  Remove:  sudo systemctl disable --now vedo-backup.timer &&"
echo "           sudo rm $SERVICE_FILE $TIMER_FILE &&"
echo "           sudo systemctl daemon-reload"
