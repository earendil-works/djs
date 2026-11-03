# 0174: REPL and CLI: tools, waiting inputs, continuation dumps

status: open
agent: -
branch: issue/0174-repl-continuations
depends-on: 0171 (the tools and pending-call part can land before it)
touches: crates/djs-cli/src/repl/, crates/djs-cli/src/run.rs, crates/djs-cli/src/main.rs, crates/djs-cli/tests/, docs/cli.md
guide: -

## Goal
Make suspension visible and testable by hand (docs/continuation-dumps.md "REPL").

## Acceptance criteria
- [ ] A host-provided `tools`: `tools.x(args)` is an async host call answered by hand;
      inputs that wait do not block the prompt and print their result when they settle.
- [ ] `.pending`, `.resolve <id> <json>`, `.reject <id> <message>`, `.bt <id|all>`
      (suspended activations and their bindings); `--tools answers.json` for scripted answers.
- [ ] `.dump` writes format 2 when something is in flight; `.restore` restores waiting inputs
      and open calls; `.dump-every [on|off]` dumps and reloads at every idle point.
- [ ] `djs run --tools answers.json --dump-after K out.js`, `djs resume out.js --tools
      answers.json`, `djs run --dump-every-idle`.
- [ ] Line-mode transcripts in crates/djs-cli/tests/repl.rs for each command; a CLI test that
      runs `--dump-after K` / `resume` for every K of a few scripts and compares with the
      uninterrupted run. docs/cli.md updated.
- [ ] `.snapshot` stays until 0177.

## Log

## Follow-ups

## Result
