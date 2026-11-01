#!/usr/bin/env bash
# Regenerates case.txt from QuickJS (the oracle). See gen-case.js for the format.
set -euo pipefail
cd "$(dirname "$0")"
QJS="${QJS:-$HOME/Development/dquickts/qjs}"
"$QJS" gen-case.js > case.txt
wc -l case.txt
