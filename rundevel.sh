#!/usr/bin/env bash
export SQLITE_EXTENSION_DIR="/home/thbellem/workspace/workspace_rust/chimitheque_back/src/extensions"
RUST_LOG="debug" APP_URL="http://localhost:8083" DB_PATH="/home/thbellem/workspace/workspace_go/src/github.com/tbellembois/gochimitheque/chimitheque.sqlite" ISSUER="http://keycloak:8080/keycloak/realms/chimitheque" CLIENT_ID="chimitheque" CLIENT_SECRET="mysupersecret" cargo run .
