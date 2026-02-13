#!/bin/bash
# generate test coverage report using cargo-llvm-cov

set -e

# check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "cargo-llvm-cov not found, installing..."
    cargo install cargo-llvm-cov
fi

# check if llvm-tools-preview is installed
if ! rustup component list | grep -q "llvm-tools-preview.*installed"; then
    echo "Installing llvm-tools-preview..."
    rustup component add llvm-tools-preview
fi

echo ""
echo "Running tests with coverage..."
echo ""

# run coverage and show summary
cargo llvm-cov --all-features

echo ""
echo "---"
echo ""

# show per-file breakdown
echo "Per-file coverage:"
echo ""
cargo llvm-cov --all-features --json 2>/dev/null | jq -r '
  .data[0].files[] |
  select(.summary.lines.count > 0) |
  "\(.summary.lines.percent | . * 10 | round / 10)%\t\(.filename | gsub(".*/src/"; "src/"))"
' | sort -t'%' -k1 -rn

echo ""
echo "---"
echo ""

# show total
TOTAL=$(cargo llvm-cov --all-features --json 2>/dev/null | jq '.data[0].totals.lines.percent | . * 10 | round / 10')
echo "Total line coverage: ${TOTAL}%"
