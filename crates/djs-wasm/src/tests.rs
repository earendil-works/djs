//! Native tests of the export layer (the node tests in js/test cover the wasm build).

use super::*;

fn stage(bytes: &[u8]) {
    with(|s| s.staging = bytes.to_vec());
}

fn out() -> Vec<u8> {
    with(|s| s.out.clone())
}

fn out_text() -> String {
    String::from_utf8(out()).unwrap()
}

fn out_words() -> Vec<u32> {
    out()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect()
}

fn compile_body(vm: u32, params: &str, src: &str) -> u32 {
    let mut b = params.as_bytes().to_vec();
    b.extend_from_slice(src.as_bytes());
    stage(&b);
    vm_compile(vm, 1, params.len(), src.len())
}

fn call(vm: u32, f: u32, args: &[u32]) {
    let b: Vec<u8> = args.iter().flat_map(|a| a.to_le_bytes()).collect();
    stage(&b);
    assert_eq!(vm_call(vm, f, 0, args.len()), 0);
}

fn display(vm: u32, h: u32) -> String {
    vm_display(vm, h);
    out_text()
}

#[test]
fn compile_and_run() {
    let vm = vm_new(0);
    assert_ne!(vm, 0);
    let f = compile_body(vm, "", "return 6 * 7");
    assert_ne!(f, 0, "{}", out_text());
    call(vm, f, &[]);
    assert_eq!(vm_run(vm, 1_000_000), status::DONE);
    let r = vm_result(vm);
    assert_eq!(display(vm, r), "42");
    vm_handle_bits(vm, r);
    assert_eq!(out_words(), [42, 0xFFF8_0004]);
    assert_eq!(vm_result(vm), 0);
    vm_handle_free(vm, r);
    vm_free(vm);
}

#[test]
fn syntax_errors() {
    let vm = vm_new(0);
    assert_eq!(compile_body(vm, "", "return )"), 0);
    assert!(out_text().starts_with("SyntaxError"), "{}", out_text());
    vm_free(vm);
}

#[test]
fn host_calls_and_fuel() {
    let vm = vm_new(0);
    stage(b"log");
    let log = vm_new_host_function(vm, 7, 3);
    let console = vm_new_object(vm);
    stage(b"log");
    assert_eq!(vm_define(vm, console, 3, log), 0);
    let f = compile_body(
        vm,
        "console",
        "let s = 0; for (let i = 0; i < 1000; i++) s += i; console.log('sum', s); return s",
    );
    assert_ne!(f, 0, "{}", out_text());
    call(vm, f, &[console]);
    // Small fuel slices: several OUT_OF_FUEL before the host call.
    let mut slices = 0;
    let st = loop {
        let st = vm_run(vm, 100);
        if st != status::OUT_OF_FUEL {
            break st;
        }
        slices += 1;
    };
    assert!(slices > 5);
    assert_eq!(st, status::HOST_CALL);
    let argc = vm_host_call(vm);
    assert_eq!(argc, 2);
    let w = out_words();
    assert_eq!(w[1], 7);
    // The host returns a value through the wire format: the VM continues with it.
    let id = w[0];
    assert_eq!(vm_resolve_host_call(vm, id, 1, 0), 0);
    assert_eq!(vm_resolve_host_call(vm, id, 1, 0), -1);
    loop {
        match vm_run(vm, 1_000_000) {
            status::OUT_OF_FUEL => continue,
            st => {
                assert_eq!(st, status::DONE);
                break;
            }
        }
    }
    let r = vm_result(vm);
    assert_eq!(display(vm, r), "499500");
    vm_free(vm);
}

#[test]
fn host_errors_are_thrown() {
    let vm = vm_new(0);
    stage(b"f");
    let hf = vm_new_host_function(vm, 1, 1);
    // (No try/catch in the compiler yet: the error propagates out of the call.)
    let f = compile_body(vm, "f", "f(); return 'not reached'");
    assert_ne!(f, 0, "{}", out_text());
    call(vm, f, &[hf]);
    let st = vm_run(vm, 1_000_000);
    if st != status::HOST_CALL {
        let r = vm_result(vm);
        panic!("status {st}: {}", display(vm, r));
    }
    vm_host_call(vm);
    let id = out_words()[0];
    stage(b"nope");
    let err = vm_new_error(vm, 1, 4);
    assert_eq!(vm_resolve_host_call(vm, id, 0, err), 0);
    assert_eq!(vm_run(vm, 1_000_000), status::THREW);
    let r = vm_result(vm);
    assert_eq!(display(vm, r), "TypeError: nope");
    vm_free(vm);
}

#[test]
fn wire_values_and_images() {
    let vm = vm_new(0);
    // [1, "x"] in the wire format (docs/wire.md).
    stage(&[b'D', b'J', b'W', 1, 0x20, 2, 0x07, 1, 0x10, 1, 1, b'x']);
    let v = vm_decode_wire(vm, 12);
    assert_ne!(v, 0, "{}", out_text());
    let f = compile_body(vm, "v", "return v.length + v[1]");
    call(vm, f, &[v]);
    assert_eq!(vm_run(vm, 1_000_000), status::DONE);
    let r = vm_result(vm);
    assert_eq!(display(vm, r), "2x");
    // Corrupt wire data is an error, not a trap.
    stage(b"DJW\x01\x99");
    assert_eq!(vm_decode_wire(vm, 5), 0);
    assert!(out_text().contains("unknown tag"));
    // The arena is the image; a restored VM keeps the handles.
    let image = with(|s| s.slot(vm).vm.bytes().to_vec());
    assert_eq!(vm_arena_len(vm), image.len());
    stage(&image);
    let vm2 = vm_from_bytes(image.len(), 0);
    assert_ne!(vm2, 0, "{}", out_text());
    assert_eq!(display(vm2, r), "2x");
    stage(b"not an image");
    assert_eq!(vm_from_bytes(12, 0), 0);
    assert!(out_text().starts_with("cannot restore"));
    vm_free(vm);
    vm_free(vm2);
}

#[test]
fn out_of_memory_is_a_thrown_error() {
    let vm = vm_new(4 << 20);
    assert_ne!(vm, 0);
    let f = compile_body(vm, "", "let s = 'x'; for (;;) s += s; return s");
    call(vm, f, &[]);
    let st = loop {
        match vm_run(vm, 1_000_000) {
            status::OUT_OF_FUEL => continue,
            st => break st,
        }
    };
    assert_eq!(st, status::THREW);
    let r = vm_result(vm);
    let msg = display(vm, r);
    assert!(
        msg.contains("out of memory") || msg.contains("RangeError"),
        "{msg}"
    );
    vm_free(vm);
    // A cap too small for the realm.
    assert_eq!(vm_new(1024), 0);
}

#[test]
fn json_in_and_out() {
    let vm = vm_new(0);
    let text = r#"{"a": [1, "ü", null], "b": {"c": true}}"#;
    stage(text.as_bytes());
    let v = vm_parse_json(vm, text.len());
    assert_ne!(v, 0, "{}", out_text());
    let s = vm_json_stringify(vm, v);
    assert_ne!(s, 0, "{}", out_text());
    assert_eq!(display(vm, s), r#"{"a":[1,"ü",null],"b":{"c":true}}"#);
    stage(b"{\"a\": }");
    assert_eq!(vm_parse_json(vm, 7), 0);
    assert!(out_text().starts_with("SyntaxError"), "{}", out_text());
    assert_eq!(vm_json_stringify(vm, 0), 0);
    assert_eq!(out_text(), "undefined");
    vm_free(vm);
}

#[test]
fn abort_after_out_of_fuel() {
    let vm = vm_new(0);
    let f = compile_body(vm, "", "for (;;) {}");
    assert_ne!(f, 0);
    call(vm, f, &[]);
    assert_eq!(vm_run(vm, 1000), status::OUT_OF_FUEL);
    vm_abort(vm);
    assert_eq!(vm_run(vm, 1000), status::IDLE);
    let g = compile_body(vm, "", "return 6 * 7");
    call(vm, g, &[]);
    assert_eq!(vm_run(vm, 1_000_000), status::DONE);
    vm_free(vm);
}

#[test]
#[cfg(feature = "compiled-prelude")]
fn base_equals_a_compiled_realm() {
    let base = vm_new(0);
    let compiled = vm_new_compiled(0);
    assert_ne!(compiled, 0);
    assert_eq!(vm_collect(compiled), 0);
    let bytes = |vm| with(|s| s.slot(vm).vm.bytes().to_vec());
    assert!(bytes(base) == bytes(compiled));
    vm_free(base);
    vm_free(compiled);
}

#[test]
fn async_host_calls() {
    let vm = vm_new(0);
    stage(b"tool");
    let tool = vm_new_async_host_function(vm, 3, 4);
    assert_ne!(tool, 0);
    let f = compile_body(vm, "tool", "return Promise.all([tool(1), tool(2)])");
    call(vm, f, &[tool]);
    assert_eq!(vm_run(vm, 1_000_000), status::IDLE);
    assert_eq!(vm_new_host_calls(vm), 2);
    let ids = out_words();
    assert_eq!(vm_new_host_calls(vm), 0);
    for (k, &id) in ids.iter().enumerate().rev() {
        assert_eq!(vm_host_call_info(vm, id), 1);
        let w = out_words();
        assert_eq!(w[..2], [id, 3]);
        assert_eq!(w[4..6], [k as u32 + 1, 0xFFF8_0004]);
        stage(format!("{}", (k + 1) * 10).as_bytes());
        let v = vm_parse_json(vm, 2);
        assert_eq!(vm_resolve_host_call(vm, id, 1, v), 0);
        vm_handle_free(vm, v);
    }
    assert_eq!(
        vm_resolve_host_call(vm, ids[0], 1, 0),
        -1,
        "already resolved"
    );
    assert_eq!(vm_run(vm, 1_000_000), status::DONE);
    let p = vm_result(vm);
    assert_eq!(vm_promise_state(vm, p), 2);
    let r = out_words()[0];
    assert_eq!(display(vm, r), "[10,20]");
    assert_eq!(vm_promise_state(vm, r), 0);
    vm_free(vm);
}

#[test]
fn host_clock() {
    let vm = vm_new(0);
    vm_set_now(vm, 1_646_177_400_000.0);
    vm_set_time_zone_offset(vm, 120);
    let f = compile_body(
        vm,
        "",
        "const d = new Date(); return d.toISOString() + ' ' + d.getHours()",
    );
    call(vm, f, &[]);
    assert_eq!(vm_run(vm, 1_000_000), status::DONE);
    let r = vm_result(vm);
    assert_eq!(display(vm, r), "2022-03-01T23:30:00.000Z 1");
    vm_free(vm);
}
