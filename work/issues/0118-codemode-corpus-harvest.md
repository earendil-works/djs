# 0118: Harvest a redacted codemode corpus from pi sessions

status: done
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
- [x] A harvester script: finds codemode executions in the session format (read pi's
      session schema from the files and from pi-private's codemode code, read-only),
      extracts script + tool calls + results, deduplicates, buckets by what the scripts
      exercise, and selects a representative set (for example 100–300 cases).
- [x] The redaction pass as above, with a report of what was redacted per case
      (committed, without the original values).
- [x] The cases as difftest cases (codemode body + fake tools replaying the redacted
      results), expected output recorded from qjs; the djs results go into the ratchet
      with failures clustered and reported to the coordinator.
- [x] TESTING.md and the guide's testing.md mention the harvested corpus.

## Log
- Session format (read from the files, structure only, and from pi-private's coding-agent
  codemode extension):
  - the assistant entry's `toolCall` named `codemode` has `arguments.code`;
  - the matching `toolResult` has `details.calls` (name, args, status, durationMs) and
    the transcript text.
  - Nested call results are not recorded, so replaying real results is impossible: every
    case gets synthetic results of the recorded kinds, in the recorded order.
- Numbers:
  - 10,098 session files, 2,620 codemode executions, 2,525 distinct;
  - 1,222 use tools with structured or unknown results (MCP, models, edit/write, ...);
  - 637 use unfaked globals (image, models, tool discovery, load, fetch, require,
    modules);
  - 573 candidates after length limits and deduplication by token shape.
- Harvester:
  - scripts/harvest/ (extract.py, build.py, jstok.py, anon.py, README.md);
  - greedy selection by language features;
  - redaction D2281: comments dropped, tool-call arguments and shell commands scrubbed
    entirely, secrets, emails, URLs, hosts, IPs, home dirs and user names replaced,
    paths renamed, unknown words renamed by a dictionary rule.
- Review: every committed case was read by hand (the script part; the header is
  generated). Rejected 9 picks: internal tool names, private app component and file
  names, a private file format, internal function names. 4 picks were excluded because
  they were already SyntaxErrors in qjs (Python written as JS, or code that needs earlier
  stored values). An automated audit of the written cases finds no /Users paths, user
  names, emails, URLs other than example.com, IPs, long hex or key formats.
- Results: qjs completes 145 cases, 5 throw.
  - djs: 145/150 identical on both backends.
  - The 5 differences are one qjs bug: object spread of a string primitive (`{a,
    ...await tool()}`) copies nothing in qjs, against the spec; djs and node copy its
    characters. They are in the corpus ratchet.
  - Identical under --suspend-every 97 and --gc-stress.
- TESTING.md, the guide's testing.md and D2280/D2281 describe the corpus.

## Follow-ups
- qjs (the oracle) is wrong on `{...stringPrimitive}`: it copies no characters. djs is
  right. Report it to the dquickts owner if wanted. The 5 ratchet entries name it.
- Harvesting the scripts that use MCP tools, `models.*` or edit/write (about 1,800 more)
  needs synthetic structured results per tool (their declared output schemas), and
  stored values for `load`.
- Ratchet lines for these cases are long (the transcript diff). A shorter reason format
  in djs-difftest would help.

## Result
- tests/corpus/codemode-harvest/: 150 redacted cases from pi sessions, with synthetic
  tool results, and REDACTION.md (per-case report).
- scripts/harvest/: the harvester.
- 145/150 match qjs; the other 5 are a qjs bug (string spread), in the ratchet and in
  TESTING.md "Known differences".
- D2280, D2281.
- The coordinator spot-checked the cases and approved them.
