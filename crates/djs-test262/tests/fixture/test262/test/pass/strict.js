/*---
description: pass/strict
---*/
assert.sameValue((function () { return this; })(), undefined);
