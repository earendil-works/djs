
### sleep: 1 dumps, 2.7 KB raw, 1.1 KB gzip

| idea | raw KB saved | raw % | gzip KB saved | gzip % |
|---|---:|---:|---:|---:|
| E3 origins without their dead prefix (upper bound) | 0.2 | 6.8 | 0.0 | 1.3 |
| E4 origin stretches already in K (if written once) | 0.2 | 6.7 | – | – |
| E6 helpers written compactly | 0.1 | 5.3 | 0.0 | 2.3 |
| E7 $dump as $d | 0.1 | 3.4 | 0.0 | 0.6 |
| E8 $djs_ names as $j | 0.0 | 1.7 | 0.0 | 0.4 |
| E2b records without default fields | 0.0 | 1.5 | 0.0 | 1.2 |
| E12 dead frame variables' declarations (raw only) | 0.0 | 0.6 | – | – |
| E9 shorter final statement | 0.0 | 0.1 | 0.0 | 0.3 |
| E2 records as $dump.o(K, f, segs, …) with defaults | -0.0 | -0.1 | -0.0 | -2.5 |

### pi: 147 dumps, 124.3 KB raw, 65.1 KB gzip

| idea | raw KB saved | raw % | gzip KB saved | gzip % |
|---|---:|---:|---:|---:|
| E12 dead frame variables' declarations (raw only) | 8.4 | 6.7 | – | – |
| E7 $dump as $d | 7.0 | 5.6 | 0.6 | 1.0 |
| E4 origin stretches already in K (if written once) | 6.2 | 5.0 | – | – |
| E3 origins without their dead prefix (upper bound) | 5.6 | 4.5 | 1.8 | 2.8 |
| E2b records without default fields | 3.3 | 2.7 | 1.9 | 3.0 |
| E1 anonymous members as (0, f), no name fixups | 0.6 | 0.5 | 0.1 | 0.1 |
| E9 shorter final statement | 0.6 | 0.5 | 0.3 | 0.5 |
| E8 $djs_ names as $j | 0.3 | 0.3 | 0.0 | 0.1 |
| E2 records as $dump.o(K, f, segs, …) with defaults | -11.3 | -9.1 | -4.9 | -7.5 |

### v2: 146 dumps, 443.3 KB raw, 159.1 KB gzip

| idea | raw KB saved | raw % | gzip KB saved | gzip % |
|---|---:|---:|---:|---:|
| E7 $dump as $d | 17.0 | 3.8 | 1.0 | 0.6 |
| E2b records without default fields | 6.4 | 1.5 | 1.3 | 0.8 |
| E2 records as $dump.o(K, f, segs, …) with defaults | 6.3 | 1.4 | -3.8 | -2.4 |
| E8 $djs_ names as $j | 5.6 | 1.3 | 0.4 | 0.3 |
| E6 helpers written compactly | 5.1 | 1.2 | 0.6 | 0.4 |
| E12 dead frame variables' declarations (raw only) | 2.8 | 0.6 | – | – |
| E9 shorter final statement | 0.6 | 0.1 | 0.1 | 0.1 |
| E5 one continuation per (function, site) (raw only) | 0.3 | 0.1 | – | – |
| E11 repeated long strings shared (raw only) | 0.1 | 0.0 | – | – |

### corpus: 1197 dumps, 24016.8 KB raw, 5073.2 KB gzip

| idea | raw KB saved | raw % | gzip KB saved | gzip % |
|---|---:|---:|---:|---:|
| E3 origins without their dead prefix (upper bound) | 6139.0 | 25.6 | 830.3 | 16.4 |
| E11 repeated long strings shared (raw only) | 3580.3 | 14.9 | – | – |
| E1 anonymous members as (0, f), no name fixups | 702.1 | 2.9 | 27.1 | 0.5 |
| E10 duplicate function texts shared (raw only) | 647.6 | 2.7 | – | – |
| E4 origin stretches already in K (if written once) | 357.5 | 1.5 | – | – |
| E7 $dump as $d | 310.3 | 1.3 | 15.2 | 0.3 |
| E12 dead frame variables' declarations (raw only) | 235.5 | 1.0 | – | – |
| E5 one continuation per (function, site) (raw only) | 102.6 | 0.4 | – | – |
| E2b records without default fields | 77.9 | 0.3 | 19.3 | 0.4 |
| E2 records as $dump.o(K, f, segs, …) with defaults | 45.2 | 0.2 | -40.1 | -0.8 |
| E8 $djs_ names as $j | 21.9 | 0.1 | 1.2 | 0.0 |
| E6 helpers written compactly | 20.9 | 0.1 | 2.4 | 0.0 |
| E9 shorter final statement | 4.7 | 0.0 | 3.3 | 0.1 |

## corpus without the environment (--no-env)

### corpus: 1197 dumps, 8562.7 KB raw, 2541.1 KB gzip

| idea | raw KB saved | raw % | gzip KB saved | gzip % |
|---|---:|---:|---:|---:|
| E1 anonymous members as (0, f), no name fixups | 702.1 | 8.2 | 27.8 | 1.1 |
| E4 origin stretches already in K (if written once) | 357.5 | 4.2 | – | – |
| E7 $dump as $d | 304.9 | 3.6 | 16.5 | 0.6 |
| E11 repeated long strings shared (raw only) | 268.9 | 3.1 | – | – |
| E3 origins without their dead prefix (upper bound) | 254.9 | 3.0 | 59.2 | 2.3 |
| E10 duplicate function texts shared (raw only) | 247.6 | 2.9 | – | – |
| E12 dead frame variables' declarations (raw only) | 235.5 | 2.8 | – | – |
| E5 one continuation per (function, site) (raw only) | 102.6 | 1.2 | – | – |
| E2b records without default fields | 77.9 | 0.9 | 17.9 | 0.7 |
| E2 records as $dump.o(K, f, segs, …) with defaults | 45.2 | 0.5 | -39.6 | -1.6 |
| E8 $djs_ names as $j | 21.9 | 0.3 | 1.6 | 0.1 |
| E6 helpers written compactly | 20.9 | 0.2 | 2.3 | 0.1 |
| E9 shorter final statement | 4.7 | 0.1 | 2.5 | 0.1 |
