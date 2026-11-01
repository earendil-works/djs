/*---
description: pass/evalscript
---*/
$262.evalScript("var fromEval = 1;");
assert.sameValue(fromEval, 1);
$262.evalScript("let lexical = 2;");
assert.sameValue(lexical, 2);
assert.sameValue(Object.getOwnPropertyDescriptor(this, "lexical"), undefined);
assert.throws(SyntaxError, function () { $262.evalScript("var;"); });
