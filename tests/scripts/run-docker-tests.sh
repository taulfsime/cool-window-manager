#!/bin/bash
# run integration tests in Docker
# usage: ./tests/scripts/run-docker-tests.sh [options]
#
# options:
#   --no-cleanup    keep containers running after tests
#   --rebuild       force rebuild of Docker images
#   --test NAME     run only specific test
#   --debug         start debug shell instead of running tests
#   --verbose       show verbose output

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
COMPOSE_FILE="$PROJECT_ROOT/tests/docker/docker-compose.yml"

# defaults
CLEANUP=true
REBUILD=false
TEST_FILTER=""
DEBUG_MODE=false
VERBOSE=false

# parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-cleanup)
            CLEANUP=false
            shift
            ;;
        --rebuild)
            REBUILD=true
            shift
            ;;
        --test)
            TEST_FILTER="$2"
            shift 2
            ;;
        --debug)
            DEBUG_MODE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --no-cleanup    Keep containers running after tests"
            echo "  --rebuild       Force rebuild of Docker images"
            echo "  --test NAME     Run only specific test"
            echo "  --debug         Start debug shell instead of running tests"
            echo "  --verbose       Show verbose output"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

cd "$PROJECT_ROOT"

echo "=== CWM Integration Tests ==="
echo ""

# cleanup function
cleanup() {
    if [ "$CLEANUP" = true ]; then
        echo ""
        echo "=== Cleaning up ==="
        docker-compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true
    else
        echo ""
        echo "=== Containers left running (--no-cleanup) ==="
        echo "To stop: docker-compose -f $COMPOSE_FILE down -v"
    fi
}

# set trap for cleanup
trap cleanup EXIT

# rebuild if requested
if [ "$REBUILD" = true ]; then
    echo "=== Rebuilding Docker images ==="
    docker-compose -f "$COMPOSE_FILE" build --no-cache
    echo ""
fi

# start mock server
echo "=== Starting mock GitHub server ==="
docker-compose -f "$COMPOSE_FILE" up -d mock-github

# wait for mock server to be ready
echo "Waiting for mock server..."
for i in {1..30}; do
    if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
        echo "Mock server is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "Error: Mock server failed to start"
        docker-compose -f "$COMPOSE_FILE" logs mock-github
        exit 1
    fi
    sleep 1
done
echo ""

# run tests or debug shell
if [ "$DEBUG_MODE" = true ]; then
    echo "=== Starting debug shell ==="
    echo "Mock server URL: http://mock-github:8080"
    echo "Type 'exit' to quit"
    echo ""
    docker-compose -f "$COMPOSE_FILE" run --rm debug
else
    echo "=== Running integration tests ==="
    
    # build test command
    TEST_CMD="cargo test --test integration"
    
    if [ -n "$TEST_FILTER" ]; then
        TEST_CMD="$TEST_CMD $TEST_FILTER"
    fi
    
    TEST_CMD="$TEST_CMD -- --test-threads=1"
    
    if [ "$VERBOSE" = true ]; then
        TEST_CMD="$TEST_CMD --nocapture"
    fi
    
    # run tests
    docker-compose -f "$COMPOSE_FILE" run --rm test-runner $TEST_CMD
    
    TEST_EXIT_CODE=$?
    
    echo ""
    if [ $TEST_EXIT_CODE -eq 0 ]; then
        echo "=== All tests passed ==="
    else
        echo "=== Some tests failed ==="
        exit $TEST_EXIT_CODE
    fi
fi
