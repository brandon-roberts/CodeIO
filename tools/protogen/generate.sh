#!/usr/bin/env bash
# Master proto codegen — runs protoc for all 8 language targets.
# Requires: protoc, grpc plugins for each language, buf (optional lint)
# Usage: ./tools/protogen/generate.sh [--lang cpp|rust|haskell|python|java|js|php|all]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PROTO_SRC="$REPO_ROOT/proto"
GEN_OUT="$REPO_ROOT/proto/gen"

LANG="${1:-all}"

log() { echo "  [protogen] $*"; }

gen_cpp() {
  log "C++ ..."
  mkdir -p "$GEN_OUT/cpp"
  protoc \
    --proto_path="$PROTO_SRC" \
    --cpp_out="$GEN_OUT/cpp" \
    --grpc_out="$GEN_OUT/cpp" \
    --plugin=protoc-gen-grpc="$(which grpc_cpp_plugin)" \
    $(find "$PROTO_SRC" -name "*.proto" | sed "s|$PROTO_SRC/||")
}

gen_rust() {
  log "Rust (via tonic-build in build.rs — no standalone codegen needed)"
  log "Run: cargo build inside services/ to regenerate"
}

gen_haskell() {
  log "Haskell ..."
  mkdir -p "$GEN_OUT/haskell"
  # Requires proto-lens-protoc
  protoc \
    --proto_path="$PROTO_SRC" \
    --haskell_out="$GEN_OUT/haskell" \
    --plugin=protoc-gen-haskell="$(which proto-lens-protoc)" \
    $(find "$PROTO_SRC" -name "*.proto" | sed "s|$PROTO_SRC/||")
}

gen_python() {
  log "Python ..."
  mkdir -p "$GEN_OUT/python"
  python -m grpc_tools.protoc \
    --proto_path="$PROTO_SRC" \
    --python_out="$GEN_OUT/python" \
    --grpc_python_out="$GEN_OUT/python" \
    --pyi_out="$GEN_OUT/python" \
    $(find "$PROTO_SRC" -name "*.proto" | sed "s|$PROTO_SRC/||")
  # Fix relative imports produced by grpc_tools
  find "$GEN_OUT/python" -name "*_pb2_grpc.py" \
    -exec sed -i 's/^import \(.*_pb2\)/from . import \1/' {} +
}

gen_java() {
  log "Java ..."
  mkdir -p "$GEN_OUT/java"
  protoc \
    --proto_path="$PROTO_SRC" \
    --java_out="$GEN_OUT/java" \
    --grpc-java_out="$GEN_OUT/java" \
    --plugin=protoc-gen-grpc-java="$(which protoc-gen-grpc-java)" \
    $(find "$PROTO_SRC" -name "*.proto" | sed "s|$PROTO_SRC/||")
}

gen_js() {
  log "TypeScript ..."
  mkdir -p "$GEN_OUT/js"
  protoc \
    --proto_path="$PROTO_SRC" \
    --plugin="$(npm root)/.bin/protoc-gen-ts_proto" \
    --ts_proto_out="$GEN_OUT/js" \
    --ts_proto_opt=outputServices=grpc-js,esModuleInterop=true \
    $(find "$PROTO_SRC" -name "*.proto" | sed "s|$PROTO_SRC/||")
}

gen_php() {
  log "PHP ..."
  mkdir -p "$GEN_OUT/php"
  protoc \
    --proto_path="$PROTO_SRC" \
    --php_out="$GEN_OUT/php" \
    --grpc_out="$GEN_OUT/php" \
    --plugin=protoc-gen-grpc="$(which grpc_php_plugin)" \
    $(find "$PROTO_SRC" -name "*.proto" | sed "s|$PROTO_SRC/||")
}

case "$LANG" in
  cpp)     gen_cpp ;;
  rust)    gen_rust ;;
  haskell) gen_haskell ;;
  python)  gen_python ;;
  java)    gen_java ;;
  js)      gen_js ;;
  php)     gen_php ;;
  all)
    gen_cpp
    gen_rust
    gen_haskell
    gen_python
    gen_java
    gen_js
    gen_php
    ;;
  *)
    echo "Unknown language: $LANG. Use: cpp|rust|haskell|python|java|js|php|all"
    exit 1
    ;;
esac

log "Done. Generated stubs in $GEN_OUT/"
