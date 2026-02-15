#!/bin/bash
# run the same checks as CI locally
# usage: ./scripts/ci.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # no color

echo -e "${BLUE}Running CI checks...${NC}"
echo ""

echo -e "${BLUE}==> cargo test --all-features${NC}"
cargo test --all-features
echo ""

echo -e "${BLUE}==> cargo fmt -- --check${NC}"
cargo fmt -- --check
echo ""

echo -e "${BLUE}==> cargo clippy -- -D warnings${NC}"
cargo clippy -- -D warnings
echo ""

echo -e "${GREEN}✓ All CI checks passed!${NC}"
