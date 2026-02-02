#!/bin/bash
# Record demo video
# Run from this directory: ./record.sh

set -e

DEMO_DIR="/tmp/blendwerk-demo"

echo "==> Cleaning up previous demo environment..."
rm -rf "$DEMO_DIR"
mkdir -p "$DEMO_DIR/mocks/api/hello"

echo "==> Creating mock files..."
echo '{"message": "Hello World!"}' > "$DEMO_DIR/mocks/api/hello/GET.json"
printf '%s\n' '---' 'status: 201' '---' '{"received": true}' > "$DEMO_DIR/mocks/api/hello/POST.json"

echo "==> Recording demo..."
vhs demo.tape

echo "==> Moving outputs to pages assets..."
mv demo.webm demo.mp4 ../pages/assets/ 2>/dev/null || true

echo "==> Cleaning up temp directory..."
rm -rf "$DEMO_DIR"

echo "==> Done! Videos are in docs/pages/assets/"
