#!/usr/bin/env bash
# Parallel unattended run; options and examples: run it with no arguments (help text in tools/parallel.py).
exec "$(dirname "$(realpath "$0")")/parallel.py" "$@"
