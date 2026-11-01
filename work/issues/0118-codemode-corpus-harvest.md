# 0118: Harvest a redacted codemode corpus from pi sessions

status: in-progress
agent: infra
branch: issue/0118-codemode-corpus-harvest
depends-on: 0044
guide: docs/guide/testing.md (the corpus sources)
touches: scripts/ (the harvester), tests/corpus/codemode-harvest/ (new cases), crates/djs-difftest/ (only if replaying recorded tool results needs runner support), TESTING.md, work/scratch/0118/ (raw extraction, never committed)

## Goal
Owner decision (D021): harvest codemode scripts from `~/.pi/agent/sessions` (~3,750
session directories, 7.2 GB) into the difftest corpus, **redacted**: this is the most
realistic test input we can get (TESTING.md's corpus source #1), replayed with recorded
tool results through fake tools, compared against qjs.

## Privacy rules (strict)
- Raw session data never leaves work/scratch/0118/ (gitignored) and is never printed in
  full to the chat; nothing from it is committed unredacted.
- Redact before anything is committed: secrets and credentials (API keys, tokens,
  passwords, private keys, auth headers, cookies, connection strings), personal data
  (names, emails, phone numbers, addresses), user-specific paths (home directories, user
  names), hostnames/IPs/URLs of private services, and private content in recorded tool
  results (file contents, documents, messages). Prefer replacing tool results with
  synthetic data of the same shape (same JSON structure, types, sizes) over keeping real
  content. When in doubt, drop the case.
- Automated detection (regexes for key formats, high-entropy strings, emails, paths) plus
  a manual review of every committed case by the agent; the coordinator spot-checks before
  the merge.

## Acceptance criteria
- [ ] A harvester script: finds codemode executions in the session format (read pi's
      session schema from the files and from pi-private's codemode code, read-only),
      extracts script + tool calls + results, deduplicates, buckets by what the scripts
      exercise, and selects a representative set (for example 100–300 cases).
- [ ] The redaction pass as above, with a report of what was redacted per case
      (committed, without the original values).
- [ ] The cases as difftest cases (codemode body + fake tools replaying the redacted
      results), expected output recorded from qjs; the djs results go into the ratchet
      with failures clustered and reported to the coordinator.
- [ ] TESTING.md and the guide's testing.md mention the harvested corpus.

## Log

## Follow-ups

## Result
