/*---
description: fail/async-failure
flags: [async]
---*/
Promise.reject(new TypeError("nope")).then($DONE, $DONE);
