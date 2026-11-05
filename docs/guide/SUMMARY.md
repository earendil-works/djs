# Using djs

- [What djs is](introduction.md)
- [What djs optimizes for](goals.md)
- [The language](language.md)
- [TypeScript](typescript.md)
- [The JavaScript interface](js-api.md)
- [Continuation dumps](dumps.md)
- [Limits and resource control](limits.md)
- [Determinism](determinism.md)
- [The command line and the REPL](cli.md)
- [djs in pi](pi.md)

# How it works

- [One arena: all state in a byte array](arena.md)
- [Values and NaN-boxing](values.md)
- [Cells, strings and objects](heap.md)
- [The compiler: tokens to bytecode, no AST](compiler.md)
- [The interpreter: frames in the heap](interpreter.md)
- [Suspending anywhere: fuel and host calls](suspension.md)
- [The garbage collector](gc.md)
- [Images and continuation dumps](snapshots.md)
- [Builtins: the TypeScript subset and the leaf rule](builtins.md)
- [The host interface: wire in, reader out](host.md)
- [How djs is tested](testing.md)

# Reference

- [Opcodes](ref-opcodes.md)
- [Cell layouts](ref-cells.md)
- [Intrinsics](ref-intrinsics.md)
- [Status in numbers](status.md)
