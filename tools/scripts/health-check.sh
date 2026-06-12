#!/usr/bin/env bash
# Verify all gRPC services are reachable.
# Requires: grpcurl  (brew install grpcurl  /  apt install grpcurl)
set -euo pipefail

SERVICES=(
  "localhost:50052 codeio.index.WorkspaceScanService"
  "localhost:50053 codeio.ai.SpotlightService"
  "localhost:50054 codeio.ai.ContextWindowService"
  "localhost:50055 codeio.index.DependencyMapService"
)

ALL_OK=true
for entry in "${SERVICES[@]}"; do
  addr="${entry%% *}"
  svc="${entry##* }"
  if grpcurl -plaintext "$addr" list "$svc" &>/dev/null; then
    echo "  ✓  $addr  $svc"
  else
    echo "  ✗  $addr  $svc  (not reachable)"
    ALL_OK=false
  fi
done

$ALL_OK && echo "" && echo "All services healthy." || { echo ""; echo "Some services unreachable."; exit 1; }
