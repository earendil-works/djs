# Harvesting codemode scripts from pi sessions

The difftest corpus's `tests/corpus/codemode-harvest/` holds scripts that models wrote in
real pi codemode sessions, redacted (owner's decision D021; 0118, D2280, D2281).

```sh
scripts/harvest/extract.py work/scratch/0118/extract.jsonl      # ~/.pi/agent/sessions → executions
scripts/harvest/build.py work/scratch/0118/extract.jsonl OUT --count 150 \
    --exclude work/scratch/0118/exclude.txt                     # select, redact, write cases
cargo run -q -p djs-difftest -- --engine qjs --update codemode-harvest   # record qjs
```

## Privacy rules

- Session data is private. extract.py's output and everything derived from it before
  review stay in `work/scratch/0118/` (gitignored). The scripts print counts, never
  session content.
- build.py keeps only scripts whose tools return plain strings (read, bash, ls, grep,
  find) or that call none; scripts using MCP tools, `models`, `image`, tool discovery,
  `load` (values from earlier runs), `fetch`, `require` or modules are left out.
- Redaction, per case (`REDACTION.md` lists what was done, by kind and count):
  - comments are dropped;
  - the text of every tool-call argument and of every string that looks like a shell
    command is replaced entirely (the fake tools ignore their arguments);
  - in the remaining literals and identifiers: secrets, keys, tokens, connection
    strings, emails, URLs, hosts, IPs, home directories and user names are replaced,
    path components are renamed (`dir1/file2.ts`), high-entropy strings become synthetic
    text of the same shape, and every word part that is not a common English word or a
    programming term (or is on the denylist of private names in anon.py) is renamed,
    consistently within the case;
  - tool results are synthetic (the sessions do not record them): text, paths, grep
    lines, code or JSON by the call's kind, in the recorded order; a call that failed
    rejects.
- Every case is reviewed by hand before it is committed. A case that still shows
  private names, internal tools or private project details is rejected (its source id
  goes into the exclude file) and the selection runs again. When in doubt, drop it.

`source` in REDACTION.md is a hash of the session file's path plus the execution's index:
it lets whoever has the sessions find the origin and reveals nothing else.
