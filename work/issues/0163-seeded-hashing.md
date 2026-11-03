# 0163: Seeded hashing for VM atoms and Map/Set

status: open
agent: -
branch: issue/0163-seeded-hashing
depends-on: 0150
guide: docs/guide/determinism.md, strings or collections pages as affected
touches: crates/djs-core/src/heap/ (string hash, atom index of the VM's own atoms, Map/Set tables), crates/djs-core/src/vm/ (seed from the VM's deterministic seed / options), docs/decisions.md

## Goal
docs/lessons-from-pico.md lesson 5: djs's string hash is fixed and unseeded, so crafted tool
data could make atom-index or Map/Set lookups quadratic. Seed the hash per VM:
- the base image's prefix (atoms, base index, D2431) stays unseeded and build-fixed;
- the VM's own atom index and Map/Set tables use a per-VM seed, derived from the VM's
  deterministic seed when deterministic mode is on (determinism holds: same seed, same
  layout), random otherwise; the seed lives in the image;
- iteration order of Map/Set is insertion order and doesn't change;
- a test that crafted colliding keys for the unseeded hash stay linear.
No workload slower than +1%.

## Log

## Follow-ups

## Result
