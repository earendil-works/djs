# 0302: Named records for builtin closure kinds

status: open
agent: psnap
branch: issue/0302-named-builtin-records
depends-on: 0301
touches: builtins/*.ts (records), crates/djs-bgen, crates/djs-core/src/builtins, crates/djs-core/src/vm/snapshot/, docs/decisions.md
guide: -

## Goal
Builtin closures (Promise.all elements, finally data, async-from-sync iterators, ...) keep
their state in positional records that nothing names: a builtin change that reorders a
record's slots would make a snapshot continue silently wrong instead of refusing (the worst
failure mode, 0300's review). Give each record kind a schema (field names, in builtins/*.ts,
used by NewRecord/RecordGet by name through bgen) and write records by name in snapshots;
state constants a record or class holds (promise/generator states, flag bits) are named in
the same tables.

## Acceptance criteria
- [ ] Every record kind builtins make has a named schema; positional access is gone.
- [ ] Snapshots write and read builtin records and named constants by name; a missing or
      renamed field refuses with the record's kind and field.
- [ ] A test perturbs a record's field order and a state constant's value, and snapshots
      taken before still continue the same (or refuse, never miscontinue).
- [ ] No cost in the hot paths (records compile to the same slot accesses).

## Log

## Follow-ups

## Result
