#!/usr/bin/env python3
"""Instrumentation patch for a scratch copy (0143, docs/empty-heap.md; never merged):
records which base cells a run touches. Marks the holder and value of every property
read (`Heap::prop_read`) and every template entered (`Exec::enter_template`), and makes
djs-difftest write the marked arena offsets to $DJS_TOUCH_OUT at exit. Run it in the
copy's root, build djs-difftest, run one case per process, then
`cargo run -p djs-prelude --example base-families -- --touched DIR` in the copy."""
import pathlib

def patch(path, old, new):
    p = pathlib.Path(path)
    s = p.read_text()
    assert s.count(old) == 1, (path, old[:60])
    p.write_text(s.replace(old, new))

pathlib.Path("crates/djs-core/src/touch.rs").write_text('''//! 0143 scratch instrumentation: which base cells a run touches.
use core::sync::atomic::{AtomicU8, Ordering::Relaxed};
pub static TOUCH: [AtomicU8; 1 << 17] = [const { AtomicU8::new(0) }; 1 << 17];
#[inline]
pub fn mark(off: u32) {
    let i = (off / 8) as usize;
    if i < TOUCH.len() {
        TOUCH[i].store(1, Relaxed);
    }
}
pub fn mark_value(v: crate::value::Value) {
    if let Some(o) = v.as_object() {
        mark(o.offset());
    }
}
pub fn take() -> alloc::vec::Vec<u32> {
    let mut out = alloc::vec::Vec::new();
    for (i, t) in TOUCH.iter().enumerate() {
        if t.swap(0, Relaxed) != 0 {
            out.push(i as u32 * 8);
        }
    }
    out
}
''')
patch("crates/djs-core/src/lib.rs", "mod sort;\n", "mod sort;\npub mod touch;\n")
patch("crates/djs-core/src/heap/object.rs",
      "    fn prop_read(&self, o: Obj, i: u32) -> Property {\n",
      """    fn prop_read(&self, o: Obj, i: u32) -> Property {
        let r = self.prop_read0(o, i);
        crate::touch::mark(o.offset());
        match r {
            Property::Data { value, .. } => crate::touch::mark_value(value),
            Property::Accessor { getter, setter, .. } => {
                crate::touch::mark_value(getter);
                crate::touch::mark_value(setter);
            }
        }
        r
    }

    fn prop_read0(&self, o: Obj, i: u32) -> Property {
""")
patch("crates/djs-core/src/interp/run.rs",
      "    fn enter_template(&mut self, t: u32) {\n",
      "    fn enter_template(&mut self, t: u32) {\n        crate::touch::mark(t);\n")
patch("crates/djs-difftest/src/main.rs",
      "fn main() -> ExitCode {\n",
      """fn main() -> ExitCode {
    let code = main1();
    if let Ok(p) = std::env::var("DJS_TOUCH_OUT") {
        let offs: Vec<String> = djs_core::touch::take().iter().map(|o| o.to_string()).collect();
        std::fs::write(p, offs.join("\\n")).unwrap();
    }
    code
}

fn main1() -> ExitCode {
""")
