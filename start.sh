#!/bin/bash

# Constants for file paths
REDIS_CONF_PATH="./database/redis.conf"
ENV_FILE_PATH="./api/.env"

# Check for environment argument
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 [dev|prod]"
    exit 1
fi

# Check if argument is either 'dev' or 'prod'
if [ "$1" != "dev" ] && [ "$1" != "prod" ]; then
    echo "Invalid environment. Use 'dev' or 'prod'."
    exit 1
fi

# Generate random passwords
PASSWORD=$(< /dev/urandom tr -dc A-Za-z0-9 | head -c20)
JWT_KEY=$(< /dev/urandom tr -dc A-Za-z0-9 | head -c20)

# Create or overwrite redis.conf file
mkdir -p "$(dirname "$REDIS_CONF_PATH")"
echo "requirepass \"$PASSWORD\"" > "$REDIS_CONF_PATH"
chmod 666 "$REDIS_CONF_PATH"

# Create or overwrite .env file
mkdir -p "$(dirname "$ENV_FILE_PATH")"
cat <<EOF > "$ENV_FILE_PATH"
DATABASE_PASSWORD=$PASSWORD
JWT_KEY=$JWT_KEY
RUST_LOG=error,api=info
EOF
chmod 666 "$ENV_FILE_PATH"

# Start Docker Compose
if [ "$1" == "dev" ]; then
    sudo docker compose --profile dev up
else
    sudo docker compose --profile prod up
fi