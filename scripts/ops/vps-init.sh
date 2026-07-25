#!/usr/bin/env bash
# =============================================================================
# VPS Initial Setup — VEDO hub RAG Assistant
# =============================================================================
# Run this script ONCE on a fresh VPS to install all prerequisites:
# Docker, Docker Compose plugin, and configure the deploy user.
#
# Usage (run as root on the VPS):
#   curl -fsSL https://raw.githubusercontent.com/<repo>/main/scripts/ops/vps-init.sh | bash
#   # Or after cloning the repo:
#   sudo bash scripts/ops/vps-init.sh
#
# What this script does:
#   1. Creates a dedicated 'vedo' system user for deployment
#   2. Installs Docker + Docker Compose plugin
#   3. Configures Docker daemon (log rotation, buildkit)
#   4. Creates the project directory with correct ownership
#   5. Configures firewall (UFW) — ports 22, 80, 443
# =============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "  ${YELLOW}→${NC} $1"; }
ok()    { echo -e "  ${GREEN}✓${NC} $1"; }
err()   { echo -e "  ${RED}✗${NC} $1"; exit 1; }

# ── Configuration (override via environment variables) ──
DEPLOY_USER="${DEPLOY_USER:-vedo}"
DEPLOY_PATH="${DEPLOY_PATH:-/opt/vedo}"
PROJECT_NAME="${PROJECT_NAME:-vedo}"

echo "═══════════════════════════════════════════════════════════════"
echo "  VEDO hub — VPS Initial Setup"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "  Deploy user:  ${DEPLOY_USER}"
echo "  Project path: ${DEPLOY_PATH}"
echo ""
echo "  This script will install Docker and configure the system."
echo "  It requires root privileges."
echo ""

# ── Must be root ──
if [[ "$(id -u)" -ne 0 ]]; then
    err "This script must be run as root (use sudo)."
fi

# ════════════════════════════════════════════════════════════════
# 1. Create deploy user
# ════════════════════════════════════════════════════════════════

if id "${DEPLOY_USER}" &>/dev/null; then
    ok "User '${DEPLOY_USER}' already exists"
else
    info "Creating deploy user '${DEPLOY_USER}'..."
    useradd -r -m -d "/home/${DEPLOY_USER}" -s /bin/bash "${DEPLOY_USER}"
    ok "User '${DEPLOY_USER}' created"
fi

# Add to docker group (so user can run docker without sudo)
if getent group docker >/dev/null; then
    usermod -aG docker "${DEPLOY_USER}"
fi

# ── 2. Clean up stale Docker repo from previous failed runs ──
if [ -f /etc/apt/sources.list.d/docker.list ]; then
    info "Removing stale Docker apt source (will be re-created with correct OS)..."
    rm -f /etc/apt/sources.list.d/docker.list
    ok "Stale docker.list removed"
fi

# ── 3. Update system packages ──
info "Updating system packages..."
apt-get update -qq
apt-get upgrade -y -qq
ok "System packages updated"

# ── 4. Install prerequisites ──
info "Installing prerequisites..."
apt-get install -y -qq \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    ufw \
    rsync \
    git
ok "Prerequisites installed"

# ════════════════════════════════════════════════════════════════
# 5. Install Docker
# ════════════════════════════════════════════════════════════════

if command -v docker &>/dev/null; then
    ok "Docker already installed: $(docker --version)"
else
    info "Installing Docker..."

    # Detect OS: Ubuntu uses 'ubuntu' codename, Debian uses 'debian'
    OS_ID="$(. /etc/os-release && echo "${ID}")"
    case "${OS_ID}" in
        ubuntu) DOCKER_OS="ubuntu" ;;
        debian) DOCKER_OS="debian" ;;
        *)      err "Unsupported OS '${OS_ID}' — only Ubuntu and Debian are supported." ;;
    esac
    ok "Detected OS: ${OS_ID}, using Docker repo: ${DOCKER_OS}"

    install -m 0755 -d /etc/apt/keyrings

    # Add Docker's official GPG key
    curl -fsSL "https://download.docker.com/linux/${DOCKER_OS}/gpg" | \
        gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    chmod a+r /etc/apt/keyrings/docker.gpg

    # Add the repository — use VERSION_CODENAME (works on both Ubuntu and Debian)
    DISTRO_CODENAME="$(. /etc/os-release && echo "${VERSION_CODENAME}")"
    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
      https://download.docker.com/linux/${DOCKER_OS} \
      ${DISTRO_CODENAME} stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

    apt-get update -qq
    apt-get install -y -qq \
        docker-ce \
        docker-ce-cli \
        containerd.io \
        docker-buildx-plugin \
        docker-compose-plugin

    ok "Docker installed: $(docker --version)"
fi

# Verify Docker Compose plugin
if ! docker compose version &>/dev/null; then
    err "Docker Compose plugin not found — check installation"
fi
ok "Docker Compose plugin: $(docker compose version)"

# ── 6. Configure Docker daemon ──
info "Configuring Docker daemon..."
mkdir -p /etc/docker
cat > /etc/docker/daemon.json << 'DOCKEREOF'
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "20m",
    "max-file": "5"
  },
  "features": {
    "buildkit": true
  },
  "storage-driver": "overlay2"
}
DOCKEREOF

# Enable Docker on boot
systemctl enable docker
systemctl restart docker
ok "Docker daemon configured and restarted"

# Add deploy user to docker group
usermod -aG docker "${DEPLOY_USER}"
ok "User '${DEPLOY_USER}' added to docker group"

# ════════════════════════════════════════════════════════════════
# 7. Configure firewall (UFW)
# ════════════════════════════════════════════════════════════════

info "Configuring firewall (UFW)..."
ufw --force reset > /dev/null 2>&1 || true

# Default deny incoming, allow outgoing
ufw default deny incoming
ufw default allow outgoing

# Allow SSH (22), HTTP (80), HTTPS (443)
ufw allow ssh
ufw allow 80/tcp
ufw allow 443/tcp

# Enable firewall
ufw --force enable
ok "Firewall configured: SSH, HTTP, HTTPS allowed"

# ════════════════════════════════════════════════════════════════
# 8. Create project directory
# ════════════════════════════════════════════════════════════════

info "Creating project directory tree: ${DEPLOY_PATH}"
mkdir -p "${DEPLOY_PATH}/deploy/docker"
mkdir -p "${DEPLOY_PATH}/scripts"
chown -R "${DEPLOY_USER}:${DEPLOY_USER}" "${DEPLOY_PATH}"
ok "Project directory tree created and owned by '${DEPLOY_USER}'"

# ════════════════════════════════════════════════════════════════
# 9. Set up Docker network
# ════════════════════════════════════════════════════════════════

info "Creating Docker internal network (if not exists)..."
docker network inspect internal >/dev/null 2>&1 || \
    docker network create --driver bridge internal || true
ok "Docker network 'internal' ready"

# ════════════════════════════════════════════════════════════════
# 10. Summary
# ════════════════════════════════════════════════════════════════

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo -e "  ${GREEN}VPS setup complete!${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo ""
echo "  1. Add the deploy user's SSH public key to authorized_keys:"
echo "     $ sudo -u ${DEPLOY_USER} mkdir -p /home/${DEPLOY_USER}/.ssh"
echo "     $ echo '<PUBLIC_KEY>' | sudo tee -a /home/${DEPLOY_USER}/.ssh/authorized_keys"
echo "     $ sudo chmod 700 /home/${DEPLOY_USER}/.ssh"
echo "     $ sudo chmod 600 /home/${DEPLOY_USER}/.ssh/authorized_keys"
echo "     $ sudo chown -R ${DEPLOY_USER}:${DEPLOY_USER} /home/${DEPLOY_USER}/.ssh"
echo ""
echo "  2. Add GitHub secrets (see deploy.yml comments for full list):"
echo "     - DEPLOY_HOST          → ${DEPLOY_USER}@<vps-ip>"
echo "     - DEPLOY_SSH_KEY       → contents of ~/.ssh/id_ed25519"
echo "     - DEPLOY_KNOWN_HOSTS   → ssh-keyscan -H <vps-ip>"
echo "     - DEPLOY_PATH          → ${DEPLOY_PATH}"
echo "     - DEPLOY_DOMAIN        → your-domain.com"
echo "     - DEPLOY_EMAIL         → your-email@example.com"
echo ""
echo "  3. Push to main to trigger the first deployment."
echo ""
