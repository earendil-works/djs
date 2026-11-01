/*---
description: pass/host-defined
---*/
assert.sameValue($262.global, this);
assert.sameValue(typeof print, "function");
assert.sameValue(typeof $262.gc, "function");
$262.gc();
