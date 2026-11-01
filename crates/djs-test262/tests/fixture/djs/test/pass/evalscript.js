/*---
description: pass/evalscript
---*/
$262.evalScript("var fromEval = 1;");
assert.sameValue(fromEval, 1);
$262.evalScript("let lexical = 2;");
assert.sameValue(lexical, 2);
var threw = false;
try { $262.evalScript("var;"); } catch (e) { threw = e.name === "SyntaxError"; }
assert(threw, "evalScript of bad syntax throws a SyntaxError");
