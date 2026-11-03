# 0171: Loader: suspended activations, open calls, handles

status: in-progress
agent: portable
branch: issue/0171-dump-loader
depends-on: 0169, 0170
touches: crates/djs-core/src/vm/ (load_dump primitives), interp/run/suspend.rs (building a Generator from a description), host call table, tests/dump/v2/ (hand-written fixtures), docs/continuation-dumps.md
guide: -

## Goal
The loading side of format 2 dumps (docs/continuation-dumps.md "The loader primitives",
D2562), tested on hand-written dumps before any exporter exists.

## Acceptance criteria
- [ ] `$dump.resume(fn, {at, awaiting, this, args, bindings, operands, loops})` builds a
      suspended activation of `fn` at the named site from this build's suspension map (0169),
      subscribes it to `awaiting` and returns its result promise; generators and async
      generators at `yield#N` return the generator object (and an async generator's request
      queue).
- [ ] It refuses, with the reason, a site that does not exist, a binding or role or loop the
      map does not have or lacks, a value of the wrong kind (a non-promise `awaiting`).
- [ ] `$dump.cell(() => x)`, `$dump.pending(id, name, args)` (an entry in the host call
      table with the host's id), `$dump.handle(name, v)` (readable by the host after the load),
      `$dump.TDZ`.
- [ ] After the Script, the loader drains the job queue and reports the VM idle with its open
      calls and handles.
- [ ] Hand-written format 2 fixtures in tests/dump/v2/: at least the plan's example, a for-of
      with an await, every operand role, a generator, an async generator, nested activations,
      Promise.all/allSettled/any/race in flight, a rejection path. Each loads, the test
      delivers results, and the output equals the uninterrupted run's.
- [ ] Format line `// djs-dump 2`, and a decision recording the format 2 promise.

## Log

## Follow-ups

## Result
