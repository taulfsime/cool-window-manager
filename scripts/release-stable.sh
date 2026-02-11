#!/bin/bash
# Helper script to create a stable release from current HEAD

set -e

# colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { printf "${GREEN}$1${NC}\n"; }
warn() { printf "${YELLOW}$1${NC}\n"; }
error() { printf "${RED}$1${NC}\n"; }

# get current commit hash (short)
COMMIT=$(git rev-parse --short HEAD)
TAG="stable-${COMMIT}"

echo "Creating stable release for commit ${COMMIT}"
echo "Tag: ${TAG}"
echo ""

# check if tag already exists
if git tag -l "${TAG}" | grep -q "${TAG}"; then
    error "Error: Tag ${TAG} already exists!"
    error "Stable release for this commit already created."
    exit 1
fi

# check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    error "Error: You have uncommitted changes"
    error "Please commit or stash changes before creating a stable release."
    exit 1
fi

# verify this commit has been beta tested (warning only)
BETA_TAG="beta-${COMMIT}"
if ! git tag -l "${BETA_TAG}" | grep -q "${BETA_TAG}"; then
    warn "Warning: No beta release found for this commit (${BETA_TAG})"
    read -p "Create stable release without beta testing? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled. Create a beta release first:"
        echo "  ./scripts/release-beta.sh"
        exit 1
    fi
fi

# show what will be released
echo "This will create a stable release with the following commits:"
echo "================================================"
LAST_STABLE=$(git tag -l 'stable-*' --sort=-version:refname | head -n 1)

if [ -n "$LAST_STABLE" ]; then
    echo "Changes since last stable (${LAST_STABLE}):"
    git log --oneline "${LAST_STABLE}..HEAD"
else
    echo "First stable release. Recent commits:"
    git log --oneline -20
fi
echo "================================================"
echo ""

# show diff link
REPO_URL=$(git remote get-url origin | sed 's/git@github.com:/https:\/\/github.com\//' | sed 's/\.git$//')
if [ -n "$LAST_STABLE" ]; then
    echo "View full diff: ${REPO_URL}/compare/${LAST_STABLE}...HEAD"
fi
echo ""

# extra confirmation for stable
warn "⚠️  STABLE RELEASE - This will be the default for all users!"
echo ""
read -p "Create stable release ${TAG}? Type 'stable' to confirm: " CONFIRM
if [ "$CONFIRM" != "stable" ]; then
    echo "Cancelled."
    exit 1
fi

# create and push tag
git tag "${TAG}"
git push origin "${TAG}"

echo ""
info "✓ Stable release ${TAG} created and pushed!"
echo "Check GitHub Actions for build progress:"
echo "  ${REPO_URL}/actions"
