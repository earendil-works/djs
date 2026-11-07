/*---
description: pass/module-fixture-syntax (a fixture's early error is a resolution error of the test)
negative:
  phase: resolution
  type: SyntaxError
flags: [module]
---*/
$DONOTEVALUATE();
import './module-syntax_FIXTURE.js';
