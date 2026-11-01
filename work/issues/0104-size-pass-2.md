# 0104: wasm size pass 2

status: done
agent: num
branch: issue/0104-size-pass-2
depends-on: 0099
touches: crates/ (code-size changes anywhere, coordinated: heap is in heap/ (0098), perf in bgen/natives/hot.rs (0100), infra in js/pi + cli + error builtins (0102)), Cargo.toml profiles only with a recorded decision

## Goal
Since 0057 (code 709 KB, module 1,241 KB raw / 428 KB gzip) the module grew to ~1,515 KB raw /
531 KB gzip (0095 README): RegExp \p (+27 KB gz), normalization (+20 KB gz), natives (+80 KB
code), @fast (+24 KB), shapes (+12 KB code), JSON native (+10.5 KB), and more. D016: size
counts. Do a second measured pass with the 0052/0043 tools (scripts/wasm-size.py,
wasm-funcs.py, wasm-dwarf-lines.py, native-sizes.sh).

## Acceptance criteria
- [x] Attribution of the growth since 0057 by area, and the current top 30 functions.
- [x] Reductions that cost no measurable speed (wasm instructions on the perf workloads):
      duplicated code between @fast entries, dense.rs and natives, monomorphization over
      string widths, inlining of cold paths, table encodings. Coordinate with the areas'
      owners through the coordinator before touching their files.
- [x] Before/after: raw, gzip, brotli, code and data; no behavior change.

## Log
- Baseline: 0057's merge (02b41d00) rebuilt with this toolchain: 1,241,547 raw / 428,124 gzip /
  344,673 brotli; code 709,850, data 528,203. Now (start of 0104): 1,509,870 / 532,741 /
  428,012; code 922,759, data 582,777. Tools: scripts/wasm-size.py, wasm-funcs.py (names via
  CARGO_PROFILE_WASM_STRIP=debuginfo; `build-wasm.sh --keep-names` fails, see follow-ups),
  wasm-dwarf-lines.py (line tables), work/scratch/0104/{size.sh,perf.sh}.

### Growth since 0057, by area
Code +213K:
- natives (bgen `@native` state machines, 51 functions): +72K
- interpreter: Exec::run split into step_once 35.5K + run_hot 16.4K + class_op 7.3K (+7K net
  over the old run); call_intrinsic +5K; json_regexp_intrinsic 14.3K; Exec total +44K
- Rust leaf builtins (@fast `builtins::fast`, dense, string, math, json source records): ~+30K
  (and code moved out of the interpreter)
- compiler: +29K (resolve::emit_function 13.7K absorbed resolve::finish; parse_primary +4.9K;
  decorators/TS metadata)
- heap/shapes: +17K (prop_insert/remove, shape_child, dictionary mode)
- unicode normalization: +3K code
Data +55K: normalization tables ~34K, base image +8K (395 → 403K), the rest tables/strings.
Data today: base image 403K (bytecode 98K, strings 79K, 861 FuncTemplates 69K, ...), unicode
property tables ~58K, normalization ~34K.

### Top 30 functions (after D2020)
| bytes | function |
| ---: | --- |
| 37508 | <interp::run::Exec>::call_intrinsic |
| 35523 | <interp::run::Exec>::step_once |
| 16438 | <interp::run::Exec>::run_hot |
| 14317 | <interp::run::Exec>::json_regexp_intrinsic |
| 13679 | compiler::resolve::emit_function |
| 8646 | <compiler::parser::Parser>::parse_primary |
| 7882 | regexp::exec::resume |
| 7308 | <interp::run::Exec>::class_op |
| 7156 | json::parse::parse |
| 5935 | <regexp::parser::Parser>::alternative |
| 5686 | <compiler::parser::Parser>::parse_for_in_of |
| 5571 | <compiler::parser::Parser>::parse_class_element |
| 5497 | builtins::gen::natives::native_SortList |
| 5025 | <compiler::parser::Parser>::parse_assign |
| 4975 | builtins::gen::natives::native_SerializeJSONProperty |
| 4777 | <heap::Heap>::verify |
| 4644 | <dlmalloc::dlmalloc::Dlmalloc<dlmalloc::sys::System>>::malloc |
| 4640 | <compiler::parser::Parser>::parse_statement |
| 4616 | <interp::run::Exec>::call |
| 4572 | <compiler::parser::Parser>::parse_class |
| 4487 | <regexp::compiler::Compiler>::node |
| 4044 | <compiler::parser::Parser>::finish_unit |
| 3803 | json::parse::parse_utf8 |
| 3728 | builtins::date::parse::<core::iter::adapters::flatten::FlatMap<heap::string::Chunks, djs_core::heap::string::Units, <djs_core::heap::string::StrView>::units>> |
| 3604 | <compiler::parser::Parser>::parse_pattern |
| 3250 | <compiler::parser::Parser>::ts_meta_list |
| 3236 | builtins::date::format |
| 3227 | <interp::run::Exec>::regexp_replace_raw |
| 3121 | json::stringify::stringify_fast_at |
| 3067 | <compiler::parser::Parser>::ts_skip_type2 |

### Measured
- wasm-opt (binaryen 133): -Os raw -76K but gzip +10K, brotli +12K; -Oz similar. Rejected.
- D2020: drop @native from 13 @fast-covered builtins (keep push): -26.2K raw.
- D2021: number parsers one-byte only: -3.2K raw.
- Tried and reverted (bigger, not smaller): a non-generic `with` in djs-wasm (dyn closure:
  +5.3K), non-generic unicode case walks (+0.3K).
- Perf workloads (22, wasm instructions, min of 2): all within noise (±0.9%, pushpop re-run 3x).

## Follow-ups
For the owners (coordinator routes):
- perf (interp/run, after 0100): call_intrinsic 37.5K, step_once 35.5K, json_regexp_intrinsic
  14.3K: diffuse inlining (Value tag tests, Result plumbing, conv) rather than one hot spot;
  move cold arms out of line, measured on the perf workloads.
- perf (bgen natives): 71K for 51 natives. Each state inlines its conversions (ThisToString,
  ToString, ClampPosition, ...) and literal strings (`n_str("...")`); shared out-of-line
  snippets for the common inlined helpers would cut most of the repetition. A native should
  only exist where a benchmark needs it (D1634, D2020's rule).
- heap (0098 list): cold panics for cell_kind (measured -1.1K), the string-kind panics in
  heap/string.rs and the handle/atom asserts; base image layout (FuncTemplate 80 B x 861,
  Str8 headers) is 27% of the module.
- infra: `scripts/build-wasm.sh --keep-names` fails (strip=false keeps debug info, which
  --compress-relocations rejects); `CARGO_PROFILE_WASM_STRIP=debuginfo` works (scripts/perf
  setup.sh uses it).

## Result
djs-wasm 1,509,870 -> 1,480,481 raw (-29.4K), gzip 532,741 -> 527,872, brotli 428,012 ->
423,738, code 922,759 -> 893,869, data 582,777 -> 582,345; perf workloads unchanged. D2020
(no @native where @fast covers, except push) and D2021 (one-byte number parsers). The growth
since 0057 is attributed above; the remaining large items are in owned areas (follow-ups).
