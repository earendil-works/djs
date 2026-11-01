/*---
description: fail/negative-wrong-phase
negative:
  phase: parse
  type: SyntaxError
---*/
throw new SyntaxError("late");
