/*---
description: pass/host-defined
---*/
assert.sameValue($262.global, this);
var d = Object.getOwnPropertyDescriptor(this, "$262");
assert(d.writable && d.configurable && !d.enumerable);
d = Object.getOwnPropertyDescriptor(this, "print");
assert(d.writable && d.configurable && !d.enumerable);
