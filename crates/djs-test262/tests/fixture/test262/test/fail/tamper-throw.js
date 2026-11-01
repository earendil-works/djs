/*---
description: fail/tamper-throw
---*/
Array.prototype[Symbol.iterator] = function () { throw 1; };
Object.prototype.toJSON = function () { throw 2; };
String.prototype.charCodeAt = function () { throw 3; };
JSON.stringify = null;
String = null;
throw new Test262Error("after tampering \u00e9\n");
