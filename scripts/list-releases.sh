#!/bin/bash
# Helper script to list all releases

# colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo "=== Current Releases ==="
echo ""

# current commit
CURRENT=$(git rev-parse --short HEAD)
echo "Current HEAD: ${CURRENT}"
echo ""

# dev releases
echo -e "${CYAN}Dev Releases (last 3):${NC}"
DEV_TAGS=$(git tag -l 'dev-*' --sort=-version:refname | head -3)
if [ -z "$DEV_TAGS" ]; then
    echo "  (none)"
else
    echo "$DEV_TAGS" | while read tag; do
        DATE=$(git log -1 --format=%ai "${tag}" 2>/dev/null | cut -d' ' -f1)
        COMMIT=${tag#dev-}
        if [ "$COMMIT" = "$CURRENT" ]; then
            echo -e "  ${tag} (${DATE}) ${GREEN}← current${NC}"
        else
            echo "  ${tag} (${DATE})"
        fi
    done
fi
echo ""

# beta releases
echo -e "${YELLOW}Beta Releases:${NC}"
BETA_TAGS=$(git tag -l 'beta-*' --sort=-version:refname)
if [ -z "$BETA_TAGS" ]; then
    echo "  (none)"
else
    echo "$BETA_TAGS" | while read tag; do
        DATE=$(git log -1 --format=%ai "${tag}" 2>/dev/null | cut -d' ' -f1)
        COMMIT=${tag#beta-}
        if [ "$COMMIT" = "$CURRENT" ]; then
            echo -e "  ${tag} (${DATE}) ${GREEN}← current${NC}"
        else
            echo "  ${tag} (${DATE})"
        fi
    done
fi
echo ""

# stable releases
echo -e "${GREEN}Stable Releases:${NC}"
STABLE_TAGS=$(git tag -l 'stable-*' --sort=-version:refname)
if [ -z "$STABLE_TAGS" ]; then
    echo "  (none)"
else
    echo "$STABLE_TAGS" | while read tag; do
        DATE=$(git log -1 --format=%ai "${tag}" 2>/dev/null | cut -d' ' -f1)
        COMMIT=${tag#stable-}
        if [ "$COMMIT" = "$CURRENT" ]; then
            echo -e "  ${tag} (${DATE}) ${GREEN}← current${NC}"
        else
            echo "  ${tag} (${DATE})"
        fi
    done
fi
echo ""

# deprecated releases
DEPRECATED_TAGS=$(git tag -l 'deprecated-*' --sort=-version:refname)
if [ -n "$DEPRECATED_TAGS" ]; then
    echo -e "${RED}Deprecated Releases:${NC}"
    echo "$DEPRECATED_TAGS" | while read tag; do
        DATE=$(git log -1 --format=%ai "${tag}" 2>/dev/null | cut -d' ' -f1)
        echo "  ${tag} (${DATE})"
    done
    echo ""
fi

# show repo URL
REPO_URL=$(git remote get-url origin 2>/dev/null | sed 's/git@github.com:/https:\/\/github.com\//' | sed 's/\.git$//')
if [ -n "$REPO_URL" ]; then
    echo "GitHub Releases: ${REPO_URL}/releases"
fi
