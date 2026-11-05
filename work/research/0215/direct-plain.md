# Writing plain dumps directly: a plan (0215 item 2)

## Today

The Dumper (dump.rs, dump_flight.rs) walks the heap and writes **lines** shaped like
format 2's: `$dump.nN = <data>;`, `$dump.sN = (() => { let x = …; return {…}; })();`
factories, `$dump.resume(...)`-style activation lines, patches (`$dump.n2.a = …;`) and
finals. Alongside it fills `plain::Ir`: the factories with their scopes, variables and
members, the activations, and which line writes which factory or activation
(`Ir::lines`). References not yet known are `\u{1}`placeholders, resolved after the walk
(`resolve_later`). Data-only lines also get a skeleton (`Dumper::skeleton`), which the
check uses to leave them out (D2888).

plain::write (dump_plain.rs) then reads those lines back:
1. `read_items`: one item per line, classified by its text (`classify`: `$dump.X =` a
   definition, `let/const/var/class` a declaration, `function` a declaration, else a
   statement). Factories and activations come from `Ir::lines`.
2. `schedule`: each item's dependencies come from **scanning its text** (`keys_in` ->
   `scan`: a small JavaScript tokenizer that skips strings, templates, regexps,
   comments and function bodies, and collects `$dump.X[.k]` references and the globals
   the dump declares). Activations get theirs from their slots' values, placed texts,
   etc., scanned the same way.
3. Placement, the continuations (dump_cont), emission (`emit_list`, which scans again
   for `keys_in`), and the builtins a dump reads (a substring search since 0215).

Cost on a data-heavy dump (records, after 0215's indexing): the scans ~30%, the
Dumper's rendering ~20%, the rest of schedule ~10%, classification and line copying a
few %. On the corpus, plain::write is ~20% of a dump.

## Plan

Rendering already knows every reference it writes: `Dumper::expr` returns a node's
location (`$dump.nN`, `$dump.sN.key`), a builtin's (`$dump.bN`) or a root's name.
Recording them as they are written makes the read-back scans unnecessary.

**Step 1 (no change to the output, small):** items with known references.
- The Dumper keeps, for each line of `out`/`patches`/`finals`, an `ItemInfo`: its kind
  (definition of `X`, declaration of names, function declaration, statement, factory,
  activation) and its eager references as `Key`s (`Loc`, `Factory(name, key)`,
  `Global`), collected by `expr` into the current line's set (a stack, for nested
  renders that emit lines of their own: `emit_group`, patches). References inside
  function texts need nothing: function texts are the original source, which names no
  dump location. Data placeholders (`LATER`) are recorded with their node and resolved
  to keys with the text.
- plain::write takes `&[ItemInfo]` with the lines; `read_items` uses the kind instead of
  `classify`, and `schedule` uses the keys instead of `keys_of(text)`. Activations'
  slot values are Dumper expressions too: their keys are recorded with them (`Ir`'s
  `Act::slots`), so `keys_of(&sl.value)` goes too.
- What still scans: texts the writer makes itself (factory scope texts it places into
  continuation blocks, `placed_text`, the eager part of factories: `scope_eager`).
  Those are Dumper-written as well (variables' inits, members): the same recording
  gives them keys in `plain::Var`/`Prop`, so the remaining scans are of the
  continuations' own text, which dump_cont writes.
- A debug assertion (tests, difftest) compares recorded keys with `scan`'s over the
  corpus until it is trusted; then `scan` is used only for K texts.
- Expected: data-heavy dumps -25-30%, corpus dumps -5-10%.
- Files: dump.rs (`expr`, line emission), dump_flight.rs (activation lines), the IR
  structs and `read_items`/`schedule`/`emit_list` in dump_plain.rs. dump_cont.rs not
  touched.

**Step 2 (optional, larger):** no format-2 lines at all. The Dumper produces items
(`Def { name, expr }`, `Patch`, `Decl`, …) with their keys, and plain::write renders the
final text from them. That removes the line syntax (`$dump.resume(...)`-shaped
activation lines exist only to be read back), most of `LATER` resolution, and the
skeleton (the check leaves out data items directly). Gains beyond step 1 are small
(copying and classifying lines, a few %): mostly a cleanup. Worth doing only together
with other work on that boundary.

## Sequencing

Step 1 touches dump_plain.rs's schedule and read side (mine since 0215), dump.rs's
`expr` and line emission, and the IR structs pjs's continuation code reads (`Var`,
`Prop`, `Act` gain a key list; nothing changes for dump_cont except the added fields).
It can go in between pjs's items once item 4's record work has settled.
