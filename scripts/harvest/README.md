# Harvesting codemode scripts from pi sessions

The difftest corpus's `tests/corpus/codemode-harvest/` holds scripts that models wrote in
real pi codemode sessions, redacted (owner's decision D021; 0118, D2280, D2281; 0121,
D2283, D2284).

```sh
scripts/harvest/extract.py work/scratch/0121/extract.jsonl      # ~/.pi/agent/sessions → executions
scripts/harvest/build.py work/scratch/0121/extract.jsonl OUT --count 150 --start 301 \
    --exclude work/scratch/0121/exclude.txt \
    --skip tests/corpus/codemode-harvest/REDACTION.md           # select, redact, write cases
cargo run -q -p djs-difftest -- --engine qjs --update codemode-harvest   # record qjs
```

`--skip` leaves out the sources a report already lists (the committed cases), `--start`
numbers the new cases after them. Append the new rows of OUT/REDACTION.md to the
committed report as a new round.

## Fake tools

The sessions record each nested call's name, arguments and status, never its result, so
every result is synthetic, in the recorded call order; a failed call rejects.
- String tools (read, bash, ls, grep, find, edit, write, transcribe_file): text of the
  call's kind (file text, command output, grep lines, code, JSON). bash returns
  `{ exit_code, output, truncated }` when the session declared that shape or the script
  reads those fields.
- Other tools (MCP): extract.py collects the TypeScript declarations the session saw
  (`toolsAdded` descriptions and `describeTool` output). synth.py parses a tool's
  result type (unions, intersections, arrays, tuples, object and index types, `Record`,
  generics such as `CallToolResult<T>`, literals) and builds a value of that type.
  Object keys of the declaration are kept when they are dictionary words, as the
  script reads them; string values are synthetic. A run that calls an MCP tool without
  a declared result type, or whose type synth.py does not read, is left out.
- `models`: getModelsOfType/getAvailableOfType/getModelOfType return synthetic model
  descriptors, classify answers each question by its type (choice, score, boolean),
  generateImages returns an image block.
- `image()` prints `[image]`.
- `load(key)`: only literal keys whose value the session recorded with `store`
  (`codemode-store` entries); the case gets a synthetic value of the same shape. A
  value the script stored in the same run is returned as is.
- tool discovery (searchTools, describeTool, describeNamespace), `fetch`, `require` and
  modules are not faked: such runs are left out.

## Privacy rules

- Session data is private. extract.py's output and everything derived from it before
  review stay in `work/scratch/<issue>/` (gitignored). The scripts print counts, never
  session content.
- build.py keeps only scripts whose tools it can fake (above). Runs of tools whose name
  contains `admin` are left out.
- Redaction, per case (`REDACTION.md` lists what was done, by kind and count):
  - comments are dropped;
  - the text of every tool-call argument, of every string that looks like a shell
    command and of every string literal longer than 40 characters (prompts,
    instructions, queries, embedded programs) is replaced entirely, letter by letter
    (the fake tools ignore their arguments);
  - ids and slugs that mix letters and digits (12 characters or more) become synthetic
    text (`opaque-id`);
  - in the remaining literals and identifiers: secrets, keys, tokens, connection
    strings, emails, URLs, hosts, IPs, home directories and user names are replaced,
    path components are renamed (`dir1/file2.ts`), high-entropy strings become synthetic
    text of the same shape, and every word part that is not a common English word or a
    programming term (or is on the denylist of private names in anon.py) is renamed,
    consistently within the case; words with non-ASCII letters are always renamed;
  - an MCP tool's server prefix is renamed like any other word (`mcp__x__name` becomes
    `qal__x__name` unless x is a dictionary word); REDACTION.md lists the tool calls
    under the names the case uses;
  - tool results are synthetic (above).
- Every case is reviewed by hand before it is committed. A case that still shows
  private names, internal tools or private project details is rejected (its source id
  goes into the exclude file) and the selection runs again. When in doubt, drop it.

`source` in REDACTION.md is a hash of the session file's path plus the execution's index:
it lets whoever has the sessions find the origin and reveals nothing else.
