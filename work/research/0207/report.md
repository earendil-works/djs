## sleep: 1 dumps, 2.7 KB raw, 1.1 KB gzip -9 (each dump alone)

per dump raw: median 2714, p90 2714, max 2714; gzip median 1079, p90 1079

| category | raw KB | raw % | marginal gzip KB | gzip % | dumps with it | median share |
|---|---:|---:|---:|---:|---:|---:|
| **cont** | 0.7 | 27.0 | 0.2 | 15.8 | 1 | 27.0 |
| &nbsp; cont.blank | 0.0 | 0.0 | | | | |
| &nbsp; cont.code | 0.4 | 15.7 | | | | |
| &nbsp; cont.head | 0.0 | 0.9 | | | | |
| &nbsp; cont.indent | 0.0 | 0.1 | | | | |
| &nbsp; cont.scaf | 0.3 | 10.3 | | | | |
| **helpers** | 0.5 | 18.4 | 0.2 | 17.8 | 1 | 18.4 |
| **origin** | 0.4 | 16.1 | 0.0 | 3.7 | 1 | 16.1 |
| **record** | 0.4 | 14.8 | 0.2 | 16.4 | 1 | 14.8 |
| &nbsp; record.at | 0.0 | 1.1 | | | | |
| &nbsp; record.cmap | 0.1 | 3.0 | | | | |
| &nbsp; record.f | 0.0 | 1.0 | | | | |
| &nbsp; record.scaf | 0.1 | 2.5 | | | | |
| &nbsp; record.segs | 0.1 | 3.0 | | | | |
| &nbsp; record.wrap | 0.1 | 4.2 | | | | |
| **fn** | 0.1 | 5.6 | 0.0 | 0.3 | 1 | 5.6 |
| **final** | 0.1 | 3.0 | 0.0 | 2.8 | 1 | 3.0 |
| **iterator** | 0.1 | 2.9 | 0.0 | 1.8 | 1 | 2.9 |
| **activation** | 0.1 | 2.8 | 0.0 | 1.4 | 1 | 2.8 |
| **hostcall** | 0.1 | 2.7 | 0.0 | 3.1 | 1 | 2.7 |
| **builtin** | 0.1 | 2.4 | 0.0 | 2.1 | 1 | 2.4 |
| **seed** | 0.0 | 1.4 | 0.0 | 2.8 | 1 | 1.4 |
| **operand** | 0.0 | 1.3 | 0.0 | 0.6 | 1 | 1.3 |
| **header** | 0.0 | 1.0 | 0.0 | 1.5 | 1 | 1.0 |
| **data** | 0.0 | 0.6 | 0.0 | 0.5 | 1 | 0.6 |

lines by provenance: repeat 0.3 KB (11.6%), prelude 0.0 KB (0.5%), script 0.0 KB (0.0%), generated 2.3 KB (87.9%)

repeated lines (repeat<first): origin<cont 0.2 KB (6.3%), origin<fn 0.1 KB (3.8%), cont<fn 0.0 KB (0.9%), cont<cont 0.0 KB (0.6%)

largest dumps:

- examples/sleep-dump/out/sleep.dump.js: 2714 B raw, 1079 gzip; helpers 18.4%, origin 16.1%, cont.code 15.7%, cont.scaf 10.3%

## v2: 146 dumps, 443.3 KB raw, 159.1 KB gzip -9 (each dump alone)

per dump raw: median 2609, p90 5639, max 10911; gzip median 998, p90 1873

| category | raw KB | raw % | marginal gzip KB | gzip % | dumps with it | median share |
|---|---:|---:|---:|---:|---:|---:|
| **cont** | 93.7 | 21.1 | 20.6 | 13.0 | 144 | 20.1 |
| &nbsp; cont.blank | 0.2 | 0.0 | | | | |
| &nbsp; cont.code | 50.4 | 11.4 | | | | |
| &nbsp; cont.head | 4.8 | 1.1 | | | | |
| &nbsp; cont.indent | 1.3 | 0.3 | | | | |
| &nbsp; cont.scaf | 37.0 | 8.4 | | | | |
| **fn** | 89.7 | 20.2 | 20.5 | 12.9 | 144 | 17.8 |
| &nbsp; fn.global | 89.7 | 20.2 | | | | |
| **record** | 78.1 | 17.6 | 28.7 | 18.1 | 144 | 16.9 |
| &nbsp; record.at | 6.0 | 1.3 | | | | |
| &nbsp; record.cmap | 19.0 | 4.3 | | | | |
| &nbsp; record.f | 4.3 | 1.0 | | | | |
| &nbsp; record.scaf | 12.2 | 2.8 | | | | |
| &nbsp; record.segs | 14.3 | 3.2 | | | | |
| &nbsp; record.wrap | 22.3 | 5.0 | | | | |
| **helpers** | 46.4 | 10.5 | 18.2 | 11.4 | 146 | 6.3 |
| **fixup** | 18.6 | 4.2 | 3.8 | 2.4 | 78 | 1.2 |
| &nbsp; fixup.other | 18.6 | 4.2 | | | | |
| **hostcall** | 18.4 | 4.2 | 5.4 | 3.4 | 146 | 2.9 |
| **reenact** | 16.8 | 3.8 | 4.9 | 3.1 | 90 | 2.5 |
| **activation** | 16.2 | 3.7 | 3.2 | 2.0 | 144 | 3.3 |
| **globals** | 14.4 | 3.2 | 4.9 | 3.1 | 110 | 2.4 |
| **data** | 12.3 | 2.8 | 3.6 | 2.3 | 118 | 1.8 |
| **final** | 11.7 | 2.6 | 3.8 | 2.4 | 146 | 3.2 |
| **builtin** | 5.8 | 1.3 | 1.6 | 1.0 | 61 | 0.0 |
| **iterator** | 5.7 | 1.3 | 1.7 | 1.0 | 41 | 0.0 |
| **seed** | 5.6 | 1.3 | 4.1 | 2.6 | 146 | 1.5 |
| **operand** | 5.1 | 1.2 | 1.5 | 1.0 | 104 | 0.9 |
| **header** | 4.0 | 0.9 | 2.3 | 1.4 | 146 | 1.1 |
| **patch** | 0.5 | 0.1 | 0.2 | 0.1 | 8 | 0.0 |
| **group** | 0.4 | 0.1 | 0.1 | 0.0 | 16 | 0.0 |
| **layout** | 0.0 | 0.0 | 0.0 | 0.0 | 16 | 0.0 |

lines by provenance: repeat 24.4 KB (5.5%), prelude 2.0 KB (0.5%), script 93.2 KB (21.0%), generated 323.8 KB (73.0%)

repeated lines (repeat<first): cont<fn 22.0 KB (5.0%), cont<cont 0.8 KB (0.2%), cont<globals 0.8 KB (0.2%), fn<fn 0.7 KB (0.2%), globals<fn 0.1 KB (0.0%), globals<globals 0.0 KB (0.0%)

largest dumps:

- v2/operands/1.js: 10911 B raw, 2661 gzip; fn.global 14.7%, cont.scaf 13.4%, record.cmap 9.8%, record.wrap 9.6%
- v2/operands/2.js: 10426 B raw, 2617 gzip; fn.global 15.4%, cont.scaf 12.7%, record.wrap 9.5%, record.cmap 9.4%
- v2/operands/3.js: 9711 B raw, 2460 gzip; fn.global 16.5%, cont.scaf 11.8%, record.wrap 9.6%, record.cmap 9.0%
- v2/operands/4.js: 9200 B raw, 2404 gzip; fn.global 17.4%, cont.scaf 11.0%, record.wrap 9.5%, cont.code 9.0%
- v2/operands/5.js: 8744 B raw, 2322 gzip; fn.global 18.3%, cont.scaf 10.3%, record.wrap 9.3%, cont.code 9.2%

## corpus: 1197 dumps, 24016.8 KB raw, 5073.2 KB gzip -9 (each dump alone)

per dump raw: median 14756, p90 29783, max 178663; gzip median 3682, p90 6194

| category | raw KB | raw % | marginal gzip KB | gzip % | dumps with it | median share |
|---|---:|---:|---:|---:|---:|---:|
| **env** | 15454.2 | 64.3 | 2532.1 | 49.9 | 1197 | 60.4 |
| &nbsp; env.data | 25.7 | 0.1 | | | | |
| &nbsp; env.fixup | 53.8 | 0.2 | | | | |
| &nbsp; env.fn | 6271.0 | 26.1 | | | | |
| &nbsp; env.harness | 9103.6 | 37.9 | | | | |
| **cont** | 1701.9 | 7.1 | 179.0 | 3.5 | 1197 | 5.2 |
| &nbsp; cont.blank | 0.2 | 0.0 | | | | |
| &nbsp; cont.code | 1475.3 | 6.1 | | | | |
| &nbsp; cont.head | 29.6 | 0.1 | | | | |
| &nbsp; cont.indent | 41.6 | 0.2 | | | | |
| &nbsp; cont.scaf | 155.3 | 0.6 | | | | |
| **fixup** | 1065.3 | 4.4 | 108.8 | 2.1 | 1197 | 6.1 |
| &nbsp; fixup.freeze | 127.5 | 0.5 | | | | |
| &nbsp; fixup.name | 921.9 | 3.8 | | | | |
| &nbsp; fixup.other | 15.9 | 0.1 | | | | |
| **hostcall** | 874.7 | 3.6 | 87.4 | 1.7 | 1197 | 2.4 |
| **origin** | 873.8 | 3.6 | 28.2 | 0.6 | 1150 | 3.6 |
| **data** | 735.3 | 3.1 | 165.2 | 3.3 | 1197 | 3.4 |
| &nbsp; data.scopevar | 334.1 | 1.4 | | | | |
| **globals** | 585.3 | 2.4 | 198.5 | 3.9 | 1197 | 3.4 |
| **record** | 518.1 | 2.2 | 168.3 | 3.3 | 1197 | 2.3 |
| &nbsp; record.at | 45.1 | 0.2 | | | | |
| &nbsp; record.cmap | 82.1 | 0.3 | | | | |
| &nbsp; record.f | 47.6 | 0.2 | | | | |
| &nbsp; record.scaf | 83.8 | 0.3 | | | | |
| &nbsp; record.segs | 81.8 | 0.3 | | | | |
| &nbsp; record.wrap | 177.6 | 0.7 | | | | |
| **reenact** | 477.6 | 2.0 | 50.9 | 1.0 | 1197 | 0.8 |
| **operand** | 382.7 | 1.6 | 51.6 | 1.0 | 1196 | 0.7 |
| **fn** | 367.0 | 1.5 | 59.3 | 1.2 | 1197 | 2.1 |
| **scope** | 354.5 | 1.5 | 79.7 | 1.6 | 1197 | 2.0 |
| &nbsp; scope.other | 3.7 | 0.0 | | | | |
| &nbsp; scope.wrap | 350.8 | 1.5 | | | | |
| **final** | 141.5 | 0.6 | 51.6 | 1.0 | 1197 | 0.8 |
| **helpers** | 135.8 | 0.6 | 53.3 | 1.1 | 1197 | 0.2 |
| **patch** | 115.0 | 0.5 | 28.6 | 0.6 | 689 | 0.6 |
| **activation** | 96.7 | 0.4 | 23.9 | 0.5 | 1197 | 0.4 |
| **seed** | 45.9 | 0.2 | 35.3 | 0.7 | 1197 | 0.3 |
| **iterator** | 37.4 | 0.2 | 7.9 | 0.2 | 185 | 0.0 |
| **header** | 32.7 | 0.1 | 19.7 | 0.4 | 1197 | 0.2 |
| **builtin** | 13.1 | 0.1 | 4.9 | 0.1 | 190 | 0.0 |
| **group** | 8.0 | 0.0 | 1.7 | 0.0 | 327 | 0.0 |
| **layout** | 0.4 | 0.0 | -0.0 | -0.0 | 327 | 0.0 |
| **other** | 0.1 | 0.0 | 0.0 | 0.0 | 3 | 0.0 |

lines by provenance: repeat 3604.2 KB (15.0%), prelude 3719.9 KB (15.5%), script 6181.7 KB (25.7%), generated 10512.3 KB (43.8%)

repeated lines (repeat<first): env<env 2025.1 KB (8.4%), env<cont 660.1 KB (2.7%), env<origin 459.3 KB (1.9%), origin<cont 351.1 KB (1.5%), cont<cont 98.6 KB (0.4%), cont<env 9.5 KB (0.0%), origin<fn 0.3 KB (0.0%), origin<origin 0.1 KB (0.0%)

largest dumps:

- corpus/codemode-harvest/h160-bash-structured-models-read.js/2.js: 178663 B raw, 18025 gzip; env.harness 87.3%, hostcall 3.7%, env.fn 3.1%, cont.code 2.1%
- corpus/codemode-harvest/h160-bash-structured-models-read.js/3.js: 178476 B raw, 17969 gzip; env.harness 87.4%, hostcall 3.6%, env.fn 3.1%, cont.code 2.1%
- corpus/codemode-harvest/h160-bash-structured-models-read.js/4.js: 178422 B raw, 17950 gzip; env.harness 87.5%, hostcall 3.1%, env.fn 3.1%, cont.code 2.1%
- corpus/codemode-harvest/h160-bash-structured-models-read.js/5.js: 178372 B raw, 17950 gzip; env.harness 87.5%, env.fn 3.1%, hostcall 2.9%, cont.code 2.1%
- corpus/codemode-harvest/h160-bash-structured-models-read.js/6.js: 178297 B raw, 17928 gzip; env.harness 87.5%, env.fn 3.1%, hostcall 2.4%, cont.code 2.1%

## pi: 147 dumps, 124.3 KB raw, 65.1 KB gzip -9 (each dump alone)

per dump raw: median 838, p90 1197, max 1301; gzip median 456, p90 574

| category | raw KB | raw % | marginal gzip KB | gzip % | dumps with it | median share |
|---|---:|---:|---:|---:|---:|---:|
| **cont** | 26.8 | 21.5 | 6.6 | 10.2 | 147 | 19.8 |
| &nbsp; cont.blank | 0.0 | 0.0 | | | | |
| &nbsp; cont.code | 22.6 | 18.2 | | | | |
| &nbsp; cont.head | 1.7 | 1.4 | | | | |
| &nbsp; cont.indent | 0.0 | 0.0 | | | | |
| &nbsp; cont.scaf | 2.4 | 2.0 | | | | |
| **origin** | 23.7 | 19.0 | 6.1 | 9.3 | 147 | 16.6 |
| **record** | 21.5 | 17.3 | 12.6 | 19.4 | 147 | 18.4 |
| &nbsp; record.at | 2.2 | 1.7 | | | | |
| &nbsp; record.cmap | 2.1 | 1.7 | | | | |
| &nbsp; record.f | 2.0 | 1.6 | | | | |
| &nbsp; record.scaf | 3.9 | 3.1 | | | | |
| &nbsp; record.segs | 3.1 | 2.5 | | | | |
| &nbsp; record.wrap | 8.2 | 6.6 | | | | |
| **hostcall** | 14.5 | 11.6 | 6.1 | 9.3 | 147 | 10.5 |
| **final** | 11.6 | 9.4 | 4.1 | 6.3 | 147 | 9.7 |
| **operand** | 6.2 | 5.0 | 1.5 | 2.3 | 147 | 5.0 |
| **seed** | 5.0 | 4.0 | 3.5 | 5.3 | 147 | 4.3 |
| **activation** | 4.2 | 3.3 | 1.4 | 2.1 | 147 | 3.5 |
| **header** | 4.0 | 3.2 | 2.2 | 3.4 | 147 | 3.3 |
| **helpers** | 3.9 | 3.1 | 1.5 | 2.3 | 147 | 3.0 |
| **reenact** | 2.1 | 1.7 | 0.6 | 0.9 | 27 | 0.0 |
| **fixup** | 0.7 | 0.6 | 0.1 | 0.2 | 6 | 0.0 |
| &nbsp; fixup.name | 0.7 | 0.6 | | | | |
| **group** | 0.1 | 0.1 | 0.0 | 0.0 | 6 | 0.0 |
| **data** | 0.1 | 0.1 | 0.0 | 0.0 | 6 | 0.0 |
| **layout** | 0.0 | 0.0 | 0.0 | 0.0 | 6 | 0.0 |

lines by provenance: repeat 3.9 KB (3.1%), prelude 2.1 KB (1.7%), script 0.0 KB (0.0%), generated 118.4 KB (95.3%)

repeated lines (repeat<first): origin<cont 3.9 KB (3.1%)

largest dumps:

- pi/10621-1793820474343-7748h6.js: 1301 B raw, 549 gzip; cont.code 16.5%, reenact 15.1%, origin 14.8%, hostcall 11.8%
- pi/10621-1793820474348-1cwn9v.js: 1301 B raw, 549 gzip; cont.code 16.5%, reenact 15.1%, origin 14.8%, hostcall 11.8%
- pi/10621-1793820475322-10e1ge.js: 1301 B raw, 549 gzip; cont.code 16.5%, reenact 15.1%, origin 14.8%, hostcall 11.8%
- pi/10621-1793820475327-acuv09.js: 1301 B raw, 549 gzip; cont.code 16.5%, reenact 15.1%, origin 14.8%, hostcall 11.8%
- pi/10621-1793820475435-fixdso.js: 1299 B raw, 548 gzip; cont.code 16.6%, reenact 15.1%, origin 14.7%, hostcall 11.9%

## all: 1491 dumps, 24587.1 KB raw, 5298.5 KB gzip -9 (each dump alone)

per dump raw: median 13882, p90 28058, max 178663; gzip median 3523, p90 5804

| category | raw KB | raw % | marginal gzip KB | gzip % | dumps with it | median share |
|---|---:|---:|---:|---:|---:|---:|
| **env** | 15454.2 | 62.9 | 2532.1 | 47.8 | 1197 | 59.1 |
| &nbsp; env.data | 25.7 | 0.1 | | | | |
| &nbsp; env.fixup | 53.8 | 0.2 | | | | |
| &nbsp; env.fn | 6271.0 | 25.5 | | | | |
| &nbsp; env.harness | 9103.6 | 37.0 | | | | |
| **cont** | 1823.1 | 7.4 | 206.4 | 3.9 | 1489 | 6.3 |
| &nbsp; cont.blank | 0.5 | 0.0 | | | | |
| &nbsp; cont.code | 1548.7 | 6.3 | | | | |
| &nbsp; cont.head | 36.1 | 0.1 | | | | |
| &nbsp; cont.indent | 42.9 | 0.2 | | | | |
| &nbsp; cont.scaf | 195.0 | 0.8 | | | | |
| **fixup** | 1084.6 | 4.4 | 112.7 | 2.1 | 1281 | 5.7 |
| &nbsp; fixup.freeze | 127.5 | 0.5 | | | | |
| &nbsp; fixup.name | 922.6 | 3.8 | | | | |
| &nbsp; fixup.other | 34.5 | 0.1 | | | | |
| **hostcall** | 907.6 | 3.7 | 98.9 | 1.9 | 1491 | 2.7 |
| **origin** | 897.9 | 3.7 | 34.3 | 0.6 | 1298 | 3.6 |
| **data** | 747.6 | 3.0 | 168.8 | 3.2 | 1322 | 3.2 |
| &nbsp; data.scopevar | 334.1 | 1.4 | | | | |
| **record** | 618.1 | 2.5 | 209.8 | 4.0 | 1489 | 2.5 |
| &nbsp; record.at | 53.3 | 0.2 | | | | |
| &nbsp; record.cmap | 103.3 | 0.4 | | | | |
| &nbsp; record.f | 53.9 | 0.2 | | | | |
| &nbsp; record.scaf | 100.0 | 0.4 | | | | |
| &nbsp; record.segs | 99.3 | 0.4 | | | | |
| &nbsp; record.wrap | 208.2 | 0.8 | | | | |
| **globals** | 599.7 | 2.4 | 203.4 | 3.8 | 1307 | 3.2 |
| **reenact** | 496.5 | 2.0 | 56.4 | 1.1 | 1314 | 0.7 |
| **fn** | 456.9 | 1.9 | 79.7 | 1.5 | 1342 | 2.1 |
| &nbsp; fn.global | 89.7 | 0.4 | | | | |
| **operand** | 394.1 | 1.6 | 54.6 | 1.0 | 1448 | 0.8 |
| **scope** | 354.5 | 1.4 | 79.7 | 1.5 | 1197 | 1.9 |
| &nbsp; scope.other | 3.7 | 0.0 | | | | |
| &nbsp; scope.wrap | 350.8 | 1.4 | | | | |
| **helpers** | 186.6 | 0.8 | 73.1 | 1.4 | 1491 | 0.3 |
| **final** | 164.9 | 0.7 | 59.5 | 1.1 | 1491 | 0.9 |
| **activation** | 117.1 | 0.5 | 28.5 | 0.5 | 1489 | 0.5 |
| **patch** | 115.5 | 0.5 | 28.8 | 0.5 | 697 | 0.0 |
| **seed** | 56.5 | 0.2 | 42.9 | 0.8 | 1491 | 0.3 |
| **iterator** | 43.2 | 0.2 | 9.5 | 0.2 | 227 | 0.0 |
| **header** | 40.7 | 0.2 | 24.2 | 0.5 | 1491 | 0.2 |
| **builtin** | 18.9 | 0.1 | 6.6 | 0.1 | 252 | 0.0 |
| **group** | 8.4 | 0.0 | 1.7 | 0.0 | 349 | 0.0 |
| **layout** | 0.4 | 0.0 | -0.0 | -0.0 | 349 | 0.0 |
| **other** | 0.1 | 0.0 | 0.0 | 0.0 | 3 | 0.0 |

lines by provenance: repeat 3632.8 KB (14.8%), prelude 3724.0 KB (15.1%), script 6274.8 KB (25.5%), generated 10956.9 KB (44.6%)

repeated lines (repeat<first): env<env 2025.1 KB (8.2%), env<cont 660.1 KB (2.7%), env<origin 459.3 KB (1.9%), origin<cont 355.2 KB (1.4%), cont<cont 99.4 KB (0.4%), cont<fn 22.1 KB (0.1%), cont<env 9.5 KB (0.0%), cont<globals 0.8 KB (0.0%)

