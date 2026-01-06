#!/usr/bin/env bash
export SQLITE_EXTENSION_DIR="/home/thbellem/workspace/workspace_rust/chimitheque_back/src/extensions"
RUST_LOG="debug" DB_PATH="/home/thbellem/workspace/workspace_go/src/github.com/tbellembois/gochimitheque/chimitheque.sqlite" KEYCLOAK_BASE_URL="https://192.168.1.18:8443/keycloak" KEYCLOAK_REDIRECT_URL="http://192.168.1.18:8083/auth/callback" KEYCLOAK_REALM="chimitheque" CLIENT_ID="chimitheque" cargo run .
