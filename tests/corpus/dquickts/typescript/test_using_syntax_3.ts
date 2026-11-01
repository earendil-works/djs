// Imported from dquickts tests/test_typescript.ts (test_using_syntax_3) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// assertSyntaxError #3 of test_using: must be a SyntaxError.

function f() { using { x }: Disposable = null; }
