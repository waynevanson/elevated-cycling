#!/bin/sh

# read and combine contents, uncompress, untar
cat "$@" | xz -d -c | tar xf -
