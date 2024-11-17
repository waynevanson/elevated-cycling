#!/bin/sh
cat parts/* | xz -d -c | tar xf -
