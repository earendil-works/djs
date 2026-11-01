// Imported from dquickts tests/test_typescript.ts (test_const_enums_syntax_1) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// assertSyntaxError #1 of test_const_enums: must be a SyntaxError.

let x = 1; const enum Bad { A = x }
