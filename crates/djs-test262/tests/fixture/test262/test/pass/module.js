/*---
description: pass/module (live bindings across fixtures, a self import, a fixture one directory down)
flags: [module]
---*/
import { count, inc, selfNs } from './module-dep_FIXTURE.js';
import * as self from './module.js';
import { up } from './sub/module-sub_FIXTURE.js';
export const here = 1;
assert.sameValue(count, 0);
inc();
assert.sameValue(count, 1);
assert.sameValue(self.here, 1);
assert.sameValue(selfNs, self);
assert.sameValue(up, 1);
assert.sameValue(this, undefined);
