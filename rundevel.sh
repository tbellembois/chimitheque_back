#!/usr/bin/env bash
export CHIMITHEQUE_URL="http://localhost/app/"
export CHIMITHEQUE_PATH="/"
export SQLITE_EXTENSION_DIR="/home/thbellem/workspace/workspace_rust/chimitheque_back/src/extensions"
RUST_LOG="debug" APP_URL="http://localhost/back/" DB_PATH="/home/thbellem/workspace/workspace_go/src/github.com/tbellembois/gochimitheque/chimitheque.sqlite" ISSUER="http://localhost/keycloak/realms/chimitheque" CLIENT_ID="chimitheque" CLIENT_SECRET="mysupersecret" cargo run .
