//! A dispatch-cost model of djs's interpreter (0081): the same bytecode loop (the one djs
//! compiles `for (let i = 0; i < n; i++) s += 1;` to, 17 ops per iteration) over one
//! `Vec<u8>` arena with u32 offsets and NaN-boxed values, in three styles:
//!
//! - `a`: djs today: the state (pc, sp, fuel, code, lp, stack) in `Exec` fields, the
//!   arena behind `&mut Heap`, two-sided checked `first_chunk` accesses, and a
//!   non-inlined slow path taking `&mut self` in the loop.
//! - `b`: the state in locals, the arena as a local `&mut [u8]` re-derived after the
//!   slow path, one-sided checked accesses. Safe Rust, still bounds checked.
//! - `c`: `b` with unchecked accesses (what D1260's "feature flag" would give).
//!
//! `model <a|b|c> <iterations>`; measure with /usr/bin/time -l (instructions retired).
#![allow(clippy::all)]

const TAG_INT: u64 = 0xFFF8_0004 << 32;
const TAG_BOOL: u64 = 0xFFF8_0003 << 32;
fn int(v: i32) -> u64 {
    TAG_INT | v as u32 as u64
}
fn as_int(v: u64) -> Option<i32> {
    if v >> 32 == TAG_INT >> 32 { Some(v as u32 as i32) } else { None }
}

mod op {
    pub const GET_LOC: u8 = 0;
    pub const PUT_LOC: u8 = 1;
    pub const SET_LOC: u8 = 2;
    pub const PUSH_I32: u8 = 3;
    pub const LT: u8 = 4;
    pub const JUMP_IF_FALSE: u8 = 5;
    pub const JUMP: u8 = 6;
    pub const ADD: u8 = 7;
    pub const DROP: u8 = 8;
    pub const INC: u8 = 9;
    pub const DUP: u8 = 10;
    pub const TO_NUMERIC: u8 = 11;
    pub const PUSH_I8: u8 = 12;
    pub const HALT: u8 = 13;
}

fn program(n: i32) -> Vec<u8> {
    use op::*;
    let mut c = Vec::new();
    let loc = |c: &mut Vec<u8>, o: u8, l: u16| {
        c.push(o);
        c.extend_from_slice(&l.to_le_bytes());
    };
    // locals: 0 = s, 1 = i
    let top = c.len();
    loc(&mut c, GET_LOC, 1);
    c.push(PUSH_I32);
    c.extend_from_slice(&n.to_le_bytes());
    c.push(LT);
    let jf = c.len();
    c.push(JUMP_IF_FALSE);
    c.extend_from_slice(&0i32.to_le_bytes());
    let jb = c.len();
    c.push(JUMP);
    c.extend_from_slice(&0i32.to_le_bytes());
    let upd = c.len();
    loc(&mut c, GET_LOC, 1);
    c.push(TO_NUMERIC);
    c.push(DUP);
    c.push(INC);
    loc(&mut c, PUT_LOC, 1);
    c.push(DROP);
    let j = c.len();
    c.push(JUMP);
    c.extend_from_slice(&((top as i32) - (j as i32 + 5)).to_le_bytes());
    let body = c.len();
    loc(&mut c, GET_LOC, 0);
    c.push(PUSH_I8);
    c.push(1);
    c.push(ADD);
    loc(&mut c, SET_LOC, 0);
    c.push(DROP);
    let j2 = c.len();
    c.push(JUMP);
    c.extend_from_slice(&((upd as i32) - (j2 as i32 + 5)).to_le_bytes());
    let end = c.len();
    c.push(HALT);
    let patch = |c: &mut Vec<u8>, at: usize, to: usize| {
        let d = to as i32 - (at as i32 + 5);
        c[at + 1..at + 5].copy_from_slice(&d.to_le_bytes());
    };
    patch(&mut c, jf, end);
    patch(&mut c, jb, body);
    c
}

// ---- the arena (as djs: a Vec<u8> with a zeroed frontier, u32 offsets) -----------------

struct Arena {
    bytes: Vec<u8>,
}

#[cold]
#[inline(never)]
fn oob() -> ! {
    panic!("arena access out of bounds")
}

impl Arena {
    // djs's read_array: `bytes.get(off..).and_then(first_chunk)` (two compares).
    #[inline(always)]
    fn read_a<const N: usize>(&self, off: u32) -> [u8; N] {
        match self.bytes.get(off as usize..).and_then(<[u8]>::first_chunk::<N>) {
            Some(a) => *a,
            None => oob(),
        }
    }
    #[inline(always)]
    fn write_a<const N: usize>(&mut self, off: u32, v: [u8; N]) {
        match self.bytes.get_mut(off as usize..).and_then(<[u8]>::first_chunk_mut::<N>) {
            Some(a) => *a = v,
            None => oob(),
        }
    }
}

const CODE: u32 = 64; // code cell data offset
const STACK: u32 = 4096; // stack cell offset; values from STACK + 16

struct Heap {
    arena: Arena,
    gc_pending: bool,
}

// ---- style a: djs today ----------------------------------------------------------------

struct Exec<'h> {
    heap: &'h mut Heap,
    fuel: i64,
    stack: u32,
    sp: u32,
    pc: u32,
    code: u32,
    code_len: u32,
    lp: u32,
}

impl Exec<'_> {
    #[inline(always)]
    fn get(&self, i: u32) -> u64 {
        u64::from_le_bytes(self.heap.arena.read_a::<8>(self.stack + 16 + i * 8))
    }
    #[inline(always)]
    fn set(&mut self, i: u32, v: u64) {
        let at = self.stack + 16 + i * 8;
        self.heap.arena.write_a::<8>(at, v.to_le_bytes())
    }
    #[inline(always)]
    fn push(&mut self, v: u64) {
        self.set(self.sp, v);
        self.sp += 1;
    }
    #[inline(always)]
    fn pop(&mut self) -> u64 {
        self.sp -= 1;
        self.get(self.sp)
    }
    #[inline(always)]
    fn byte(&self, at: u32) -> u8 {
        self.heap.arena.read_a::<1>(self.code + at)[0]
    }
    #[inline(always)]
    fn imm_u16(&self) -> u32 {
        u16::from_le_bytes(self.heap.arena.read_a::<2>(self.code + self.pc + 1)) as u32
    }
    #[inline(always)]
    fn imm_i32(&self) -> i32 {
        i32::from_le_bytes(self.heap.arena.read_a::<4>(self.code + self.pc + 1))
    }
    fn run(&mut self) -> u64 {
        loop {
            if self.fuel <= 0 {
                return 0;
            }
            if self.fast_step() {
                continue;
            }
            if self.slow_step() {
                return self.get(self.lp);
            }
        }
    }
    #[inline(always)]
    fn fast_step(&mut self) -> bool {
        let pc = self.pc;
        if pc >= self.code_len {
            return false;
        }
        match self.byte(pc) {
            op::GET_LOC => {
                let v = self.get(self.lp + self.imm_u16());
                self.push(v);
                self.fuel -= 1;
                self.pc += 3;
            }
            op::PUT_LOC => {
                let v = self.pop();
                self.set(self.lp + self.imm_u16(), v);
                self.fuel -= 1;
                self.pc += 3;
            }
            op::SET_LOC => {
                let v = self.get(self.sp - 1);
                self.set(self.lp + self.imm_u16(), v);
                self.fuel -= 1;
                self.pc += 3;
            }
            op::PUSH_I32 => {
                let v = self.imm_i32();
                self.push(int(v));
                self.fuel -= 1;
                self.pc += 5;
            }
            op::PUSH_I8 => {
                let v = self.byte(pc + 1) as i8;
                self.push(int(v as i32));
                self.fuel -= 1;
                self.pc += 2;
            }
            op::LT | op::ADD => {
                let (Some(x), Some(y)) = (as_int(self.get(self.sp - 2)), as_int(self.get(self.sp - 1))) else {
                    return false;
                };
                let v = if self.byte(pc) == op::LT {
                    TAG_BOOL | (x < y) as u64
                } else {
                    match x.checked_add(y) {
                        Some(r) => int(r),
                        None => return false,
                    }
                };
                self.sp -= 1;
                self.set(self.sp - 1, v);
                self.fuel -= 1;
                self.pc += 1;
            }
            op::JUMP_IF_FALSE => {
                let v = self.get(self.sp - 1);
                if v >> 32 != TAG_BOOL >> 32 {
                    return false;
                }
                self.sp -= 1;
                self.fuel -= 1;
                let next = self.pc + 5;
                self.pc = if v & 1 == 0 { (next as i64 + self.imm_i32() as i64) as u32 } else { next };
            }
            op::JUMP => {
                self.fuel -= 1;
                let next = self.pc + 5;
                self.pc = (next as i64 + self.imm_i32() as i64) as u32;
                if self.pc < next && self.heap.gc_pending {
                    return false;
                }
            }
            op::DROP => {
                self.sp -= 1;
                self.fuel -= 1;
                self.pc += 1;
            }
            op::DUP => {
                let v = self.get(self.sp - 1);
                self.push(v);
                self.fuel -= 1;
                self.pc += 1;
            }
            op::TO_NUMERIC => {
                if as_int(self.get(self.sp - 1)).is_none() {
                    return false;
                }
                self.fuel -= 1;
                self.pc += 1;
            }
            op::INC => {
                let Some(i) = as_int(self.get(self.sp - 1)).and_then(|i| i.checked_add(1)) else {
                    return false;
                };
                self.set(self.sp - 1, int(i));
                self.fuel -= 1;
                self.pc += 1;
            }
            _ => return false,
        }
        true
    }
    /// The general path (here only HALT and the overflow cases): out of line, `&mut self`.
    #[inline(never)]
    fn slow_step(&mut self) -> bool {
        let b = self.byte(self.pc);
        if b == op::HALT {
            return true;
        }
        std::hint::black_box(&mut *self);
        panic!("slow path {b}");
    }
}

// ---- style b/c: state in locals, the arena as a local slice --------------------------------

struct Regs {
    pc: u32,
    sp: u32,
    fuel: i64,
}

#[inline(always)]
fn rd<const N: usize, const CHECKED: bool>(b: &[u8], off: u32) -> [u8; N] {
    let off = off as usize;
    if CHECKED {
        // One compare: `off + N` cannot overflow a usize for a u32 `off`.
        match b.get(off..off + N) {
            Some(s) => s.try_into().unwrap(),
            None => oob(),
        }
    } else {
        unsafe { (b.as_ptr().add(off) as *const [u8; N]).read_unaligned() }
    }
}
#[inline(always)]
fn wr<const N: usize, const CHECKED: bool>(b: &mut [u8], off: u32, v: [u8; N]) {
    let off = off as usize;
    if CHECKED {
        match b.get_mut(off..off + N) {
            Some(s) => s.copy_from_slice(&v),
            None => oob(),
        }
    } else {
        unsafe { (b.as_mut_ptr().add(off) as *mut [u8; N]).write_unaligned(v) }
    }
}

impl Exec<'_> {
    fn run_local<const CHECKED: bool>(&mut self) -> u64 {
        let mut r = Regs { pc: self.pc, sp: self.sp, fuel: self.fuel };
        let (code, lp) = (self.code, self.lp);
        let vals = self.stack + 16;
        let mut b: &mut [u8] = &mut self.heap.arena.bytes[..];
        macro_rules! get {
            ($i:expr) => {
                u64::from_le_bytes(rd::<8, CHECKED>(b, vals + ($i) * 8))
            };
        }
        macro_rules! set {
            ($i:expr, $v:expr) => {{
                let v: u64 = $v;
                wr::<8, CHECKED>(b, vals + ($i) * 8, v.to_le_bytes())
            }};
        }
        macro_rules! imm16 {
            () => {
                u16::from_le_bytes(rd::<2, CHECKED>(b, code + r.pc + 1)) as u32
            };
        }
        macro_rules! imm32 {
            () => {
                i32::from_le_bytes(rd::<4, CHECKED>(b, code + r.pc + 1))
            };
        }
        loop {
            if r.fuel <= 0 {
                break;
            }
            let op = rd::<1, CHECKED>(b, code + r.pc)[0];
            let ok = match op {
                op::GET_LOC => {
                    let v = get!(lp + imm16!());
                    set!(r.sp, v);
                    r.sp += 1;
                    r.pc += 3;
                    true
                }
                op::PUT_LOC => {
                    r.sp -= 1;
                    let v = get!(r.sp);
                    set!(lp + imm16!(), v);
                    r.pc += 3;
                    true
                }
                op::SET_LOC => {
                    let v = get!(r.sp - 1);
                    set!(lp + imm16!(), v);
                    r.pc += 3;
                    true
                }
                op::PUSH_I32 => {
                    set!(r.sp, int(imm32!()));
                    r.sp += 1;
                    r.pc += 5;
                    true
                }
                op::PUSH_I8 => {
                    let v = rd::<1, CHECKED>(b, code + r.pc + 1)[0] as i8;
                    set!(r.sp, int(v as i32));
                    r.sp += 1;
                    r.pc += 2;
                    true
                }
                op::LT | op::ADD => match (as_int(get!(r.sp - 2)), as_int(get!(r.sp - 1))) {
                    (Some(x), Some(y)) => {
                        let v = if op == op::LT {
                            Some(TAG_BOOL | (x < y) as u64)
                        } else {
                            x.checked_add(y).map(int)
                        };
                        match v {
                            Some(v) => {
                                r.sp -= 1;
                                set!(r.sp - 1, v);
                                r.pc += 1;
                                true
                            }
                            None => false,
                        }
                    }
                    _ => false,
                },
                op::JUMP_IF_FALSE => {
                    let v = get!(r.sp - 1);
                    if v >> 32 != TAG_BOOL >> 32 {
                        false
                    } else {
                        r.sp -= 1;
                        let next = r.pc + 5;
                        r.pc = if v & 1 == 0 { (next as i64 + imm32!() as i64) as u32 } else { next };
                        true
                    }
                }
                op::JUMP => {
                    let next = r.pc + 5;
                    r.pc = (next as i64 + imm32!() as i64) as u32;
                    !(r.pc < next && self_gc_pending(&r))
                }
                op::DROP => {
                    r.sp -= 1;
                    r.pc += 1;
                    true
                }
                op::DUP => {
                    let v = get!(r.sp - 1);
                    set!(r.sp, v);
                    r.sp += 1;
                    r.pc += 1;
                    true
                }
                op::TO_NUMERIC => {
                    if as_int(get!(r.sp - 1)).is_some() {
                        r.pc += 1;
                        true
                    } else {
                        false
                    }
                }
                op::INC => match as_int(get!(r.sp - 1)).and_then(|i| i.checked_add(1)) {
                    Some(i) => {
                        set!(r.sp - 1, int(i));
                        r.pc += 1;
                        true
                    }
                    None => false,
                },
                _ => false,
            };
            r.fuel -= 1;
            if !ok {
                // Sync, run the general path with `&mut self`, re-derive the slice.
                r.fuel += 1;
                self.pc = r.pc;
                self.sp = r.sp;
                self.fuel = r.fuel;
                if self.slow_step() {
                    return self.get(self.lp);
                }
                r = Regs { pc: self.pc, sp: self.sp, fuel: self.fuel };
                b = &mut self.heap.arena.bytes[..];
            }
        }
        self.pc = r.pc;
        self.sp = r.sp;
        self.fuel = r.fuel;
        0
    }
}

#[inline(always)]
fn self_gc_pending(_r: &Regs) -> bool {
    false
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let style = args[1].as_str();
    let n: i32 = args[2].parse().unwrap();
    let code = program(n);
    let mut bytes = vec![0u8; 1 << 16];
    bytes[CODE as usize..CODE as usize + code.len()].copy_from_slice(&code);
    // locals s = 0, i = 0 at lp = 6 (after a fake header), operand stack after them
    let mut heap = Heap { arena: Arena { bytes }, gc_pending: false };
    let lp = 6;
    let mut ex = Exec { heap: &mut heap, fuel: i64::MAX, stack: STACK, sp: lp + 2, pc: 0, code: CODE, code_len: code.len() as u32, lp };
    ex.set(lp, int(0));
    ex.set(lp + 1, int(0));
    let r = match style {
        "a" => ex.run(),
        "b" => ex.run_local::<true>(),
        "c" => ex.run_local::<false>(),
        _ => panic!("style a|b|c"),
    };
    println!("{} {:?}", r & 0xffff_ffff, as_int(ex.get(lp)));
}
