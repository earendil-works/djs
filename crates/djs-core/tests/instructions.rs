//! Instruction counts of work whose time the gate cannot watch (0224): building a realm
//! and compiling the prelude from source, as the base image's build and
//! `startup/vm_new compiling the prelude` do. A lookup in the compiler that got slower
//! (D3091's `intrinsic_id`, a UTF-8 check per name) made it 1.6x slower, and only a bench
//! record showed it.
//!
//! Counts are machine instructions retired by this test binary running the work
//! (`/usr/bin/time -l`, macOS), minus the same run doing nothing; they repeat to well
//! under 1% under any load. Elsewhere the test only says it did not measure.
//!
//! The ceiling is today's count plus 20%: room for the prelude and the builtins to grow.
//! A count over it is a regression unless that growth explains it; then raise the
//! ceiling with the numbers in the commit message.

use std::process::Command;

use djs_core::vm::{Vm, VmOptions};

/// Instructions per `Vm::new` in the dev profile (djs-core at opt-level 1, what the gate
/// runs) and in release: 136M and 109M at 0224, +20%. With D3091's lookup the dev count
/// was 198M. Native default builds compile the `embed` builtins too (ES modules, the
/// encoding classes; D3175): 162.0M in dev before 0248, 164.5M with its async module
/// evaluation, so the dev ceiling is that plus 5% (not 20%, which would hide another
/// D3091).
const CEILING_DEV: u64 = 172_000_000;
const CEILING_RELEASE: u64 = 131_000_000;

const REPS: u32 = 3;

/// The work, run by `prelude_compile_instructions` in a child process.
#[test]
#[ignore = "run by prelude_compile_instructions"]
fn prelude_compile_work() {
    let reps: u32 = std::env::var("DJS_INSTR_REPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for _ in 0..reps {
        std::hint::black_box(Vm::new(VmOptions::default()).unwrap());
    }
}

/// Instructions retired by this binary running `prelude_compile_work` `reps` times.
fn instructions(reps: u32) -> Option<u64> {
    let exe = std::env::current_exe().ok()?;
    let out = Command::new("/usr/bin/time")
        .arg("-l")
        .arg(exe)
        .args([
            "--exact",
            "prelude_compile_work",
            "--ignored",
            "--test-threads=1",
        ])
        .env("DJS_INSTR_REPS", reps.to_string())
        .output()
        .ok()?;
    assert!(out.status.success(), "the work failed: {out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    err.lines()
        .find(|l| l.contains("instructions retired"))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|n| n.parse().ok())
}

#[test]
fn prelude_compile_instructions() {
    // The lower of two runs each: a first run can include one-time costs of the system
    // (page faults of a fresh binary).
    let lowest = |reps| {
        let a = instructions(reps)?;
        let b = instructions(reps)?;
        Some(a.min(b))
    };
    let (Some(none), Some(work)) = (lowest(0), lowest(REPS)) else {
        eprintln!("prelude_compile_instructions: no instruction counts here (needs macOS)");
        return;
    };
    let per = work.saturating_sub(none) / u64::from(REPS);
    let ceiling = if cfg!(debug_assertions) {
        CEILING_DEV
    } else {
        CEILING_RELEASE
    };
    eprintln!("prelude_compile_instructions: {per} per Vm::new (ceiling {ceiling})");
    assert!(
        per <= ceiling,
        "building the realm and compiling the prelude took {per} instructions, ceiling \
         {ceiling} (see the module docs)"
    );
}
