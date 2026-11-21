#!/bin/sh
# Instrument only this test-owned native Chromium; preserve production arguments.
exec /browser/chrome --remote-debugging-port=0 "$@"
