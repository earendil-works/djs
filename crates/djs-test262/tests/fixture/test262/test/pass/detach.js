/*---
description: pass/detach
---*/
var b = new ArrayBuffer(8);
$262.detachArrayBuffer(b);
assert.sameValue(b.byteLength, 0);
$262.detachArrayBuffer(b);
