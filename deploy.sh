#!/bin/bash
set -e

# Configuration
APP_NAME="ksana-flow"
INSTALL_DIR="/opt/$APP_NAME"
WEB_DIR="/var/www/$APP_NAME"
SERVICE_USER="root" # Ideally create a dedicated user 'ksana'

echo "Starting deployment for $APP_NAME..."

# Check for root privileges
if [ "$EUID" -ne 0 ]; then
  echo "Please run as root"
  exit 1
fi

# 1. Update system and install dependencies
echo "Installing system dependencies..."
apt-get update
apt-get install -y curl build-essential nginx git

# 2. Install Rust (if not present)
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# 3. Install Node.js (if not present)
if ! command -v bun &> /dev/null; then
    echo "Installing bun.js..."
    curl -fsSL https://bun.sh/install | bash -
fi
# 4. Build Frontend
echo "Building Frontend..."
cd web
npm install
npm run build
cd ..

# 5. Build Backend
echo "Building Backend..."
cargo build --release --bin server

# 6. Install Artifacts
echo "Installing artifacts..."
mkdir -p $INSTALL_DIR
mkdir -p $WEB_DIR

# Copy Server Binary
cp target/release/server $INSTALL_DIR/ksana-server
chmod +x $INSTALL_DIR/ksana-server

# Copy Frontend Assets
rm -rf $WEB_DIR/*
cp -r web/dist/* $WEB_DIR/

# Copy .env if it exists in current dir
if [ -f .env ]; then
    cp .env $INSTALL_DIR/
    echo "Copied .env to install directory."
fi

# 7. Setup Systemd Service
echo "Configuring Systemd Service..."
cat > /etc/systemd/system/$APP_NAME.service <<EOF
[Unit]
Description=Ksana Flow Server
After=network.target

[Service]
Type=simple
User=$SERVICE_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/ksana-server
Restart=always
Environment="RUST_LOG=info"
EnvironmentFile=-$INSTALL_DIR/.env

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable $APP_NAME
systemctl restart $APP_NAME

# 8. Setup Nginx
echo "Configuring Nginx..."
cat > /etc/nginx/sites-available/$APP_NAME <<EOF
server {
    listen 80;
    server_name _; # Replace with your domain

    root /var/www/$APP_NAME;
    index index.html;

    # Serve Static Files (Frontend)
    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # Proxy API Requests to Backend
    location /api/ {
        proxy_pass http://127.0.0.1:3000/api/;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # Proxy WebSocket Requests
    location /ws {
        proxy_pass http://127.0.0.1:3000/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }
}
EOF

# Enable Site
ln -sf /etc/nginx/sites-available/$APP_NAME /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default
nginx -t
systemctl reload nginx

echo "Deployment Complete!"
echo "Server is running at http://localhost (or your server IP)"
