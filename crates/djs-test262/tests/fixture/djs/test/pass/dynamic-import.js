/*---
description: pass/dynamic-import (a Script's import() loads a fixture relative to the test)
flags: [async]
---*/
import('./module-async_FIXTURE.js')
  .then(function (ns) { return ns.later; })
  .then(function (v) { assert.sameValue(v, 2); })
  .then($DONE, $DONE);
