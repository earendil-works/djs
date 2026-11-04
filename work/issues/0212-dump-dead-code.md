# 0212: Exporter cleanup after format 2: dead branches, chain-test allowances

status: in-progress
agent: pjd
branch: issue/0212-dump-dead-code
depends-on: 0205, 0209
guide: -
touches: vm/dump.rs, vm/dump_flight.rs (format-2-only branches), dump chain test

## Goal
0205 and 0209 follow-ups: delete the exporter's format-2-only branches in dump.rs and
dump_flight.rs (and anything else only format 2 reached: helpers, fields, options), and drop
the chain test's `runtime_bytes` allowance if D2951 made it unnecessary. No behaviour change;
record freed bytes (size ceilings) and lower them with `--update`. pjs (dump_cont.rs,
dump_plain.rs callback parts) and pjr (dump_origin.rs, hosts; dump_cont.rs assign/decl_list)
work nearby: one small merge, rebase right before it.

## Log

## Follow-ups

## Result
