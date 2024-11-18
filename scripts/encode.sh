#!/bin/sh

# Default size
SIZE="50MB"

# Parse options
while [ "$#" -gt 0 ]; do
    case "$1" in
        -b|--bytes)
            SIZE="$2"
            shift 2
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
        *)
            break
            ;;
    esac
done

# Validate positional arguments
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 [-b SIZE] SOURCE PARTS" >&2
    exit 1
fi

SOURCE="$1"
PARTS="$2"

# Ensure destination directory exists
mkdir -p "$PARTS"

# Create tarball, compress, and split
tar cf - "$SOURCE" | xz -9 -c | split -a 8 -b "$SIZE" - "$PARTS"
