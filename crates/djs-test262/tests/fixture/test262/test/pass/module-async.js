/*---
description: pass/module-async
flags: [module, async]
---*/
import { later } from './module-async_FIXTURE.js';
later.then(function (v) { assert.sameValue(v, 2); }).then($DONE, $DONE);
