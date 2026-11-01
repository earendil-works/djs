#!/usr/bin/env bash
# Regenerates corpus.txt from QuickJS (the oracle). See gen-corpus.js for the format.
set -euo pipefail
cd "$(dirname "$0")"
QJS="${QJS:-$HOME/Development/dquickts/qjs}"
"$QJS" gen-corpus.js > corpus.txt
wc -l corpus.txt
