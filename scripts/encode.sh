#!/bin/sh
tar cf - bulk | xz -9 -c | split -b 4KB - parts/part_