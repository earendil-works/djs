
#### corpus-generator: corpus codemode/plain-fuzz/generator-capturing-for-of.js, link 1: a generator capturing a frame variable, for-of over it with an await; the difftest prelude's state (env) gets only the mechanical rules

main 8530 B raw, 2484 B gzip → minimal 7434 B raw (-13%), 2139 B gzip (-14%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -70 | -23 |
| record.layout | -201 | -128 |
| record.encoding | -18 | -9 |
| seed.next | -3 | -2 |
| inline | -88 | -22 |
| builtins.methods | -24 | 3 |
| record.encoding | -109 | -19 |
| callback | -167 | -50 |
| loop | -178 | -52 |
| reread | -18 | -5 |
| inline | -91 | -26 |
| names | -78 | -5 |
| names | -32 | -4 |
| ws | -10 | 0 |
| ws | -9 | -3 |

#### pi-largest: pi suite, largest dump; an object literal half built at the site

main 910 B raw, 502 B gzip → minimal 307 B raw (-66%), 254 B gzip (-49%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -72 | -29 |
| record.layout | -303 | -131 |
| record.encoding | -19 | -12 |
| seed.next | -3 | -2 |
| inline | -23 | -7 |
| inline | -21 | -4 |
| reread | -107 | -35 |
| inline | -21 | -10 |
| holder | -23 | -15 |
| ws | -11 | -3 |

#### pi-median: pi suite (next mode, a dump at every idle point), median size; Promise.all over an open call

main 568 B raw, 367 B gzip → minimal 315 B raw (-45%), 252 B gzip (-31%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -72 | -28 |
| record.layout | -52 | -34 |
| record.encoding | -7 | -4 |
| seed.next | 0 | 0 |
| inline | -23 | -7 |
| inline | -42 | -11 |
| inline | -21 | -10 |
| holder | -23 | -17 |
| ws | -13 | -4 |

#### pi-promise-all: pi suite: Promise.all over two open calls' then-reactions (closures of the body)

main 859 B raw, 438 B gzip → minimal 347 B raw (-60%), 236 B gzip (-46%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -72 | -26 |
| record.layout | -45 | -34 |
| record.encoding | -7 | -4 |
| seed.next | -3 | -3 |
| inline | -23 | -5 |
| callback | -191 | -67 |
| builtins.methods | -70 | -22 |
| inline | -42 | -12 |
| inline | -21 | -8 |
| holder | -23 | -17 |
| ws | -15 | -4 |

#### sleep: examples/sleep-dump (the Sandbox's codemode body), the_sleep_example_dumps

main 1247 B raw, 648 B gzip → minimal 716 B raw (-43%), 411 B gzip (-37%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -72 | -28 |
| record.layout | -161 | -88 |
| record.encoding | -14 | -6 |
| seed.next | -3 | -2 |
| inline | -23 | -6 |
| inline | -21 | -6 |
| inline | -21 | -6 |
| inline | -25 | -4 |
| holder | -21 | -12 |
| loop | -115 | -51 |
| holder | -23 | -18 |
| ws | -11 | -1 |
| inline | -21 | -9 |

#### v2-generator: tests/dump/v2/generator, link 2: a generator suspended at a yield inside a for loop, driven by an async function waiting inside a while loop

main 1583 B raw, 708 B gzip → minimal 901 B raw (-43%), 391 B gzip (-45%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -82 | -35 |
| record.layout | -218 | -128 |
| record.encoding | -107 | -50 |
| record.encoding | -18 | -8 |
| seed.next | -3 | -1 |
| decl | -15 | -9 |
| loop | -158 | -54 |
| inline | -21 | -4 |
| holder | -45 | -22 |
| ws | -15 | -6 |

#### v2-operands: tests/dump/v2/operands, the largest v2 dump: 17 activations stopped inside expressions (calls, new, members, literals, assignments, templates, super)

main 7695 B raw, 2048 B gzip → minimal 3718 B raw (-52%), 1055 B gzip (-48%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -82 | -36 |
| record.layout | -1674 | -489 |
| record.encoding | -126 | -7 |
| seed.next | -4 | -2 |
| keep | -166 | -34 |
| reread | -938 | -190 |
| unused | -30 | -8 |
| inline | -39 | -12 |
| inline | -373 | -79 |
| inline | -437 | -110 |
| inline | -12 | -11 |
| holder | -23 | -13 |
| ws | -73 | -2 |

#### v2-plan: tests/dump/v2/plan (docs/continuation-dumps.md's example): closures sharing a frame variable, a nested continuation, Promise.all, a loop

main 2535 B raw, 1031 B gzip → minimal 1682 B raw (-34%), 652 B gzip (-37%)

| step (in order) | Δ raw | Δ gzip |
|---|---:|---:|
| final | -82 | -27 |
| record.layout | -203 | -125 |
| record.encoding | -26 | -18 |
| seed.next | -3 | -2 |
| callback | -219 | -73 |
| inline | -21 | -10 |
| inline | -42 | -16 |
| loop | -153 | -57 |
| builtins.methods | -35 | -18 |
| inline | -33 | -16 |
| holder | -23 | -15 |
| ws | -13 | -2 |

#### Summary

| case | main raw | main gzip | minimal raw | minimal gzip | raw | gzip |
|---|---:|---:|---:|---:|---:|---:|
| corpus-generator | 8530 | 2484 | 7434 | 2139 | -13% | -14% |
| pi-largest | 910 | 502 | 307 | 254 | -66% | -49% |
| pi-median | 568 | 367 | 315 | 252 | -45% | -31% |
| pi-promise-all | 859 | 438 | 347 | 236 | -60% | -46% |
| sleep | 1247 | 648 | 716 | 411 | -43% | -37% |
| v2-generator | 1583 | 708 | 901 | 391 | -43% | -45% |
| v2-operands | 7695 | 2048 | 3718 | 1055 | -52% | -48% |
| v2-plan | 2535 | 1031 | 1682 | 652 | -34% | -37% |
| **total** | 23927 | 8226 | 15420 | 5390 | -36% | -34% |

#### By category, over all cases

| category | Δ raw | Δ gzip |
|---|---:|---:|
| record.layout | -2857 | -1157 |
| inline | -1505 | -411 |
| reread | -1063 | -230 |
| final | -604 | -232 |
| loop | -604 | -214 |
| callback | -577 | -190 |
| record.encoding | -451 | -137 |
| holder | -204 | -129 |
| ws | -170 | -25 |
| keep | -166 | -34 |
| builtins.methods | -129 | -37 |
| names | -110 | -9 |
| unused | -30 | -8 |
| seed.next | -22 | -14 |
| decl | -15 | -9 |
