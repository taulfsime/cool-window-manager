#!/bin/bash
# Helper script to create a beta release from current HEAD

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
TAG="beta-${COMMIT}"

# calculate CalVer from commit timestamp
TIMESTAMP=$(git log -1 --format=%ct)
CALVER=$(date -r "${TIMESTAMP}" +%Y.%-m.%-d 2>/dev/null || date -d "@${TIMESTAMP}" +%Y.%-m.%-d)
SEMVER="${CALVER}+beta.${COMMIT}"

echo "Creating beta release for commit ${COMMIT}"
echo "Tag: ${TAG}"
echo "Version: ${SEMVER}"
echo ""

# check if tag already exists
if git tag -l "${TAG}" | grep -q "${TAG}"; then
    error "Error: Tag ${TAG} already exists!"
    error "Beta release for this commit already created."
    exit 1
fi

# check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    warn "Warning: You have uncommitted changes"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled."
        exit 1
    fi
fi

# show what will be released
echo "This will create a beta release with the following commits:"
echo "================================================"
LAST_BETA=$(git tag -l 'beta-*' --sort=-version:refname | head -n 1)
LAST_STABLE=$(git tag -l 'stable-*' --sort=-version:refname | head -n 1)

if [ -n "$LAST_BETA" ]; then
    echo "Changes since last beta (${LAST_BETA}):"
    git log --oneline "${LAST_BETA}..HEAD"
elif [ -n "$LAST_STABLE" ]; then
    echo "Changes since last stable (${LAST_STABLE}):"
    git log --oneline "${LAST_STABLE}..HEAD"
else
    echo "Recent commits:"
    git log --oneline -10
fi
echo "================================================"
echo ""

# show diff link
REPO_URL=$(git remote get-url origin | sed 's/git@github.com:/https:\/\/github.com\//' | sed 's/\.git$//')
if [ -n "$LAST_BETA" ]; then
    echo "View full diff: ${REPO_URL}/compare/${LAST_BETA}...HEAD"
elif [ -n "$LAST_STABLE" ]; then
    echo "View full diff: ${REPO_URL}/compare/${LAST_STABLE}...HEAD"
fi
echo ""

# confirm
read -p "Create beta release ${TAG}? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 1
fi

# create and push tag
git tag "${TAG}"
git push origin "${TAG}"

echo ""
info "✓ Beta release ${TAG} created and pushed!"
echo "Check GitHub Actions for build progress:"
echo "  ${REPO_URL}/actions"
