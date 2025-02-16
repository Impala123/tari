#!/bin/bash
#
# Docker Start Script for Tari applications
# The docker compose environment should set the following envars
# - APP_NAME - the name of the app to run. This var is used to set the location of log files, and app-specific config
# - APP_EXEC - the name of the application executable. Just the name is enough, since the Dockerfile will put it in /usr/bin
# - CREATE_CONFIG - set to 1 if we should write a default config file if one is missing.
# - CREATE_ID - set to 1 if we should create an id file for this application if one is missing. It will be called
#               {network}_{app_name}_id.json
# - WAIT_FOR_TOR - set to 1 to place a 30 second delay at the beginning of this script.
# - TARI_NETWORK - the Tari network to configure the docker rig for
#

APP_NAME=${APP_NAME:-base_node}
APP_EXEC=${APP_EXEC:-tari_base_node}
CREATE_CONFIG=${CREATE_CONFIG:-0}
CREATE_ID=${CREATE_ID:-0}
WAIT_FOR_TOR=${WAIT_FOR_TOR:-0}
NETWORK=${TARI_NETWORK:-weatherwax}
TARI_BASE=/var/tari/$APP_NAME
CONFIG=/var/tari/config

echo "Starting $APP_NAME with following docker environment:"
echo "executable: $APP_EXEC"
echo "network: $NETWORK"
echo "CREATE_CONFIG: $CREATE_CONFIG"
echo "CREATE_ID: $CREATE_ID"
echo "WAIT_FOR_TOR: $WAIT_FOR_TOR"
echo "base folder (in container): $TARI_BASE"
echo "config folder (in container): $CONFIG"

if [[ $WAIT_FOR_TOR != 0 ]]; then
  echo "Waiting $WAIT_FOR_TOR seconds for Tor to start up"
  sleep "$WAIT_FOR_TOR"
fi

cd "$TARI_BASE" || exit 1

ARGS=()
if [[ $CREATE_CONFIG == 1 && ! -f $CONFIG/config.toml ]]; then
  echo "Creating config file."
  ARGS+=("--init")
fi

ID_FILENAME=${NETWORK}_${APP_NAME}_id.json
if [[ $CREATE_ID && ! -f $ID_FILENAME ]]; then
  echo "Creating network identity file ($ID_FILENAME)."
  ARGS+=("--create-id")
fi

if [ -n "${ARGS[0]}" ]; then
  echo "Initializing."
  $APP_EXEC -b "$TARI_BASE" -c "$CONFIG/config.toml" "${ARGS[@]}" || exit 1
fi

$APP_EXEC "$@"
