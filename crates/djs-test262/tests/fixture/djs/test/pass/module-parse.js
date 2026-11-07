/*---
description: pass/module-parse (an early error of module code)
negative:
  phase: parse
  type: SyntaxError
flags: [module]
---*/
$DONOTEVALUATE();
export { undeclared };
