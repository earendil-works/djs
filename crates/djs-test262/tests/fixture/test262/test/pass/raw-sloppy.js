/*---
description: pass/raw-sloppy
flags: [raw]
---*/
if (typeof assert !== "undefined") throw new Error("harness was loaded");
with ({}) {}
