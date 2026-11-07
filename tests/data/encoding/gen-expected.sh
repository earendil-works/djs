#!/usr/bin/env bash
# Regenerates cases.expected from node, the oracle for TextEncoder and TextDecoder (qjs has
# neither; 0244): the completion value of cases.js as a strict script.
set -euo pipefail
cd "$(dirname "$0")"
node -e '
const src = "\"use strict\";\n" + require("fs").readFileSync("cases.js", "utf8");
process.stdout.write(require("vm").runInThisContext(src) + "\n");
' > cases.expected
