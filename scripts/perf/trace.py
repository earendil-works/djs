"""lldb script: exact dynamic instruction trace of djs's run loop.

  TRACE_SKIP=<boundaries to skip> TRACE_N=<instructions> TRACE_OUT=<file> \
  lldb -b -o 'command script import trace.py' -o 'trace_run' -- djs run --body w/x.js

Stops at the run loop's fuel check (the start of every dispatched bytecode or native
transition, TRACE_BOUNDARY) after TRACE_SKIP hits, then single-steps TRACE_N machine
instructions (into every call) and writes one line per instruction: the pc and, at a
boundary, the bytecode pc and opcode byte (from Exec.pc / Exec.code, offsets 0x28/0x30,
and the arena's Vec pointer at heap+8). Analyse with trace_report.py."""
import lldb, os

def trace_run(debugger, command, result, internal_dict):
    skip = int(os.environ.get("TRACE_SKIP", "1000"))
    n = int(os.environ.get("TRACE_N", "20000"))
    out = os.environ.get("TRACE_OUT", "trace.txt")
    boundary = int(os.environ.get("TRACE_BOUNDARY", "0x10003b5c4"), 16)
    target = debugger.GetSelectedTarget()
    debugger.SetAsync(False)
    debugger.HandleCommand("settings set target.disable-aslr true")
    debugger.HandleCommand(f"breakpoint set -s djs -a {boundary:#x}")
    bp = target.GetBreakpointAtIndex(target.GetNumBreakpoints() - 1)
    bp.SetIgnoreCount(skip)
    debugger.HandleCommand("process launch")
    process = target.GetProcess()
    print("state", process.GetState(), "hits", bp.GetHitCount())
    thread = process.GetSelectedThread()
    target.BreakpointDelete(bp.GetID())
    f = open(out, "w")
    def rd(addr, size):
        e = lldb.SBError()
        b = process.ReadMemory(addr, size, e)
        return int.from_bytes(b, "little") if e.Success() else -1
    for i in range(n):
        frame = thread.GetFrameAtIndex(0)
        pc = frame.GetPC()
        if pc == boundary:
            x20 = frame.FindRegister("x20").GetValueAsUnsigned()
            heap = rd(x20 + 0x8, 8)
            arena = rd(heap + 0x8, 8)
            bpc = rd(x20 + 0x28, 4)
            code = rd(x20 + 0x30, 4)
            code_len = rd(x20 + 0x34, 4)
            op = rd(arena + code + bpc, 1) if bpc < code_len else -1
            f.write(f"{pc:x} B {bpc} {op} {code}\n")
        else:
            f.write(f"{pc:x}\n")
        thread.StepInstruction(False)
        if process.GetState() != lldb.eStateStopped:
            break
    f.close()
    process.Kill()
    print("trace written", out)

def __lldb_init_module(debugger, internal_dict):
    debugger.HandleCommand("command script add -f trace.trace_run trace_run")
