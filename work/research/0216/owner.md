### minimal cases
8 files, 15420 B raw, 5390 B gzip

| item | Δ raw | Δ gzip |
|---|---:|---:|
| seed left out (Math.random never used: state unobserved) | -269 (-1.7%) | -197 (-3.7%) |
| "use strict" left out (hosts compile dumps strict) | -112 (-0.7%) | -72 (-1.3%) |
| source by hash alone, 8 hex digits | -170 (-1.1%) | -115 (-2.1%) |
| pi: call state left out where it is the call id | -36 (-0.2%) | -23 (-0.4%) |
| no newlines between generated statements | -22 (-0.1%) | 3 (0.1%) |

### pi suite after rules.mjs
106 files, 37248 B raw, 28220 B gzip

| item | Δ raw | Δ gzip |
|---|---:|---:|
| seed left out (Math.random never used: state unobserved) | -3299 (-8.9%) | -2195 (-7.8%) |
| "use strict" left out (hosts compile dumps strict) | -1484 (-4.0%) | -947 (-3.4%) |
| source by hash alone, 8 hex digits | -3180 (-8.5%) | -2014 (-7.1%) |
| pi: call state left out where it is the call id | -1118 (-3.0%) | -806 (-2.9%) |
| no newlines between generated statements | -384 (-1.0%) | -66 (-0.2%) |

### v2 suite after rules.mjs
192 files, 371913 B raw, 140323 B gzip

| item | Δ raw | Δ gzip |
|---|---:|---:|
| seed left out (Math.random never used: state unobserved) | -6749 (-1.8%) | -5155 (-3.7%) |
| "use strict" left out (hosts compile dumps strict) | -2688 (-0.7%) | -1638 (-1.2%) |
| source by hash alone, 8 hex digits | 0 (0.0%) | 0 (0.0%) |
| pi: call state left out where it is the call id | 0 (0.0%) | 0 (0.0%) |
| no newlines between generated statements | -2175 (-0.6%) | -11 (-0.0%) |
