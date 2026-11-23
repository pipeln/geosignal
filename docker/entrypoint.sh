#!/bin/sh
set -eu

# Hugging Face Spaces makes /data available at runtime when a Storage Bucket
# is attached. Fall back to the user's home directory only when /data is
# absent, so a plain `docker run` works without a volume. A present but
# unwritable /data is a configuration error, not something to hide behind an
# ephemeral fallback.
if [ -z "${GEOSIGNAL_DATA:-}" ]; then
    if [ -d /data ]; then
        if [ ! -w /data ]; then
            echo "[geosignal] /data exists but is not writable by UID $(id -u)." >&2
            echo "[geosignal] Fix the mount ownership, or set GEOSIGNAL_DATA to a writable path." >&2
            exit 1
        fi
        GEOSIGNAL_DATA=/data
    else
        GEOSIGNAL_DATA="$HOME/data"
    fi
fi

mkdir -p "$GEOSIGNAL_DATA"
export GEOSIGNAL_DATA
echo "[geosignal] data: $GEOSIGNAL_DATA"

exec "$@"
