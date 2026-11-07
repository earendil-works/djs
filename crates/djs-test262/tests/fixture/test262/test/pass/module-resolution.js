/*---
description: pass/module-resolution (a missing export)
negative:
  phase: resolution
  type: SyntaxError
flags: [module]
---*/
$DONOTEVALUATE();
import { nope } from './module-empty_FIXTURE.js';
