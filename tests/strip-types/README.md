# strip-types cases

`*.ts` inputs and their `*.js` output from ts-blank-space (pinned in
scripts/strip-types/fetch.sh). `djs strip-types` must produce the same bytes; the
compiler test `ts_strip_corpus` checks it in the gate. Regenerate the outputs with
`scripts/strip-types/update-cases.sh` (needs node and network on first use). The full
oracle over TypeScript's sources and conformance tests is `scripts/strip-types/oracle.sh`.
