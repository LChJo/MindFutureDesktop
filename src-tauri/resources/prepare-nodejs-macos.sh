#!/bin/bash
set -e

# Node.js version to download
VERSION="24.12.0"
TARGET_DIR="nodejs-macos"

echo "==================================="
echo "Node.js Universal Binary Preparation"
echo "Version: ${VERSION}"
echo "==================================="

# Create target directory
mkdir -p "$TARGET_DIR"
cd "$TARGET_DIR"

echo ""
echo "Step 1: Downloading Node.js binaries..."
echo "  - Intel x64 architecture"
curl -LO "https://nodejs.org/dist/v${VERSION}/node-v${VERSION}-darwin-x64.tar.gz"

echo "  - Apple Silicon ARM64 architecture"
curl -LO "https://nodejs.org/dist/v${VERSION}/node-v${VERSION}-darwin-arm64.tar.gz"

echo ""
echo "Step 2: Extracting archives..."
echo "  - Extracting x64..."
tar -xzf "node-v${VERSION}-darwin-x64.tar.gz"

echo "  - Extracting arm64..."
tar -xzf "node-v${VERSION}-darwin-arm64.tar.gz"

echo ""
echo "Step 3: Creating universal binary..."
lipo -create \
  "node-v${VERSION}-darwin-x64/bin/node" \
  "node-v${VERSION}-darwin-arm64/bin/node" \
  -output node

# Verify the universal binary
echo "  - Verifying universal binary..."
lipo -info node

echo ""
echo "Step 4: Copying npm and npx scripts..."
cp "node-v${VERSION}-darwin-x64/bin/npm" .
cp "node-v${VERSION}-darwin-x64/bin/npx" .

# Copy npm library files
echo "  - Copying npm library files..."
cp -r "node-v${VERSION}-darwin-x64/lib" .

echo ""
echo "Step 5: Creating version file..."
echo "v${VERSION}" > version.txt

echo ""
echo "Step 6: Setting executable permissions..."
chmod +x node npm npx

echo ""
echo "Step 7: Cleaning up..."
rm -rf *.tar.gz "node-v${VERSION}-darwin-x64" "node-v${VERSION}-darwin-arm64"

echo ""
echo "==================================="
echo "✅ Done! Node.js universal binary ready at: $(pwd)/node"
echo ""
echo "Binary info:"
lipo -info node
echo ""
echo "Directory contents:"
ls -lh
echo "==================================="
