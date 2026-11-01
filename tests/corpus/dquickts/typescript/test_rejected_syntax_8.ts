// Imported from dquickts tests/test_typescript.ts (test_rejected_syntax_8) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// assertSyntaxError #8 of test_rejected: must be a SyntaxError.

const enum E { A = 1, B = Math.random() }
