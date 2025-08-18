#!/bin/bash
# dev.sh
set -e

./build.sh

cd rustpos
./rustpos &
RUSTPOS_PID=$!

# Cleanup function
cleanup() {
    echo "Stopping development server..."
    kill $RUSTPOS_PID 2>/dev/null
    exit
}

trap cleanup SIGINT SIGTERM

wait
