/*
 * A minimal embedding of QuickJS (Bellard's or QuickJS-NG: the same API) as a WASI
 * reactor, for js/bench/engines/engines.ts `quickjsBuilt`: one runtime and context, a
 * global string, an eval that drains the job queue, the global `__result` as UTF-8.
 * scripts/bench-engines.sh builds it with wasi-sdk against each release's sources.
 *
 * For the bridge benchmark (js/bench/engines/bridge.ts) it also moves values between the
 * host and the global `__IN` the best ways the C API offers: JSON text parsed by
 * JS_ParseJSON straight from linear memory, strings by JS_NewStringLen, bytes by
 * JS_NewArrayBufferCopy; out by JS_JSONStringify and JS_ToCStringLen, whose UTF-8 the host
 * reads in place, or a typed array's bytes by pointer. And a sync host function `tool`
 * that passes its argument out as JSON and takes its result back as JSON or a string
 * through the import `env.bench_host`.
 */
#include <stdlib.h>
#include <string.h>
#include "quickjs.h"

static JSRuntime *rt;
static JSContext *ctx;
static char *result;
static size_t result_len;
/* The UTF-8 of the last bench_get, valid until the next. */
static const char *out_str, *out_ptr;
static size_t out_len;

/* The host's side of `tool`: gets the argument's JSON, returns the result in memory it
 * allocated with `malloc` (NUL-terminated at *len), as JSON (*kind 0) or a string (1). */
__attribute__((import_module("env"), import_name("bench_host"))) char *bench_host(const char *args, size_t n, size_t *len, int *kind);

static JSValue tool(JSContext *c, JSValueConst this_val, int argc, JSValueConst *argv) {
  JSValue j = JS_JSONStringify(c, argc > 0 ? argv[0] : JS_UNDEFINED, JS_UNDEFINED, JS_UNDEFINED);
  if (JS_IsException(j)) return j;
  size_t n;
  const char *s = JS_ToCStringLen(c, &n, j);
  JS_FreeValue(c, j);
  if (!s) return JS_EXCEPTION;
  size_t len = 0;
  int kind = 0;
  char *r = bench_host(s, n, &len, &kind);
  JS_FreeCString(c, s);
  JSValue v = kind == 1 ? JS_NewStringLen(c, r, len) : JS_ParseJSON(c, r, len, "<tool>");
  free(r);
  return v;
}

__attribute__((export_name("bench_new"))) int bench_new(void) {
  rt = JS_NewRuntime();
  if (!rt) return 1;
  ctx = JS_NewContext(rt);
  if (!ctx) return 1;
  JSValue g = JS_GetGlobalObject(ctx);
  JS_SetPropertyStr(ctx, g, "tool", JS_NewCFunction(ctx, tool, "tool", 1));
  JS_FreeValue(ctx, g);
  return 0;
}

__attribute__((export_name("bench_free"))) void bench_free(void) {
  JS_FreeCString(ctx, out_str);
  out_str = NULL;
  free(result);
  result = NULL;
  JS_FreeContext(ctx);
  JS_FreeRuntime(rt);
}

__attribute__((export_name("bench_set_global"))) void bench_set_global(const char *p, size_t n) {
  JSValue g = JS_GetGlobalObject(ctx);
  JS_SetPropertyStr(ctx, g, "__DATA", JS_NewStringLen(ctx, p, n));
  JS_FreeValue(ctx, g);
}

static void set_result(JSValue v) {
  size_t n;
  const char *s = JS_ToCStringLen(ctx, &n, v);
  free(result);
  result = malloc(n + 1);
  result_len = s ? n : 0;
  if (s) memcpy(result, s, n);
  JS_FreeCString(ctx, s);
}

static int fail(void) {
  JSValue e = JS_GetException(ctx);
  set_result(e);
  JS_FreeValue(ctx, e);
  return 1;
}

/* `p` is NUL-terminated at `n` (JS_Eval needs it). */
__attribute__((export_name("bench_eval"))) int bench_eval(const char *p, size_t n) {
  JSValue v = JS_Eval(ctx, p, n, "<bench>", JS_EVAL_TYPE_GLOBAL);
  if (JS_IsException(v)) return fail();
  JS_FreeValue(ctx, v);
  for (;;) {
    JSContext *c;
    int r = JS_ExecutePendingJob(rt, &c);
    if (r < 0) return fail();
    if (r == 0) break;
  }
  JSValue g = JS_GetGlobalObject(ctx);
  JSValue r = JS_GetPropertyStr(ctx, g, "__result");
  set_result(r);
  JS_FreeValue(ctx, r);
  JS_FreeValue(ctx, g);
  return 0;
}

/* The bridge: `kind` 0 JSON text (NUL-terminated at n), 1 a string, 2 bytes (a Uint8Array).
 * Sets the global `__IN`; 0 on success. */
__attribute__((export_name("bench_put"))) int bench_put(int kind, const char *p, size_t n) {
  JSValue v;
  if (kind == 0) {
    v = JS_ParseJSON(ctx, p, n, "<in>");
  } else if (kind == 1) {
    v = JS_NewStringLen(ctx, p, n);
  } else {
    /* `new Uint8Array(buffer)`: a view, no second copy (Bellard's JS_NewTypedArray wants a
     * new.target, so through the constructor in both releases). */
    JSValue buf = JS_NewArrayBufferCopy(ctx, (const uint8_t *)p, n);
    JSValue g = JS_GetGlobalObject(ctx);
    JSValue ctor = JS_GetPropertyStr(ctx, g, "Uint8Array");
    v = JS_CallConstructor(ctx, ctor, 1, &buf);
    JS_FreeValue(ctx, ctor);
    JS_FreeValue(ctx, g);
    JS_FreeValue(ctx, buf);
  }
  if (JS_IsException(v)) return fail();
  JSValue g = JS_GetGlobalObject(ctx);
  JS_SetPropertyStr(ctx, g, "__IN", v);
  JS_FreeValue(ctx, g);
  return 0;
}

/* The global `__IN` out: `kind` 0 as JSON text, 1 a string, 2 a Uint8Array's bytes, in
 * place. The bytes are at bench_out_ptr()/bench_out_len() until the next call. */
__attribute__((export_name("bench_get"))) void bench_get(int kind) {
  JS_FreeCString(ctx, out_str);
  out_str = NULL;
  out_ptr = NULL;
  out_len = 0;
  JSValue g = JS_GetGlobalObject(ctx);
  JSValue v = JS_GetPropertyStr(ctx, g, "__IN");
  JS_FreeValue(ctx, g);
  if (kind == 2) {
    size_t off, len, per, size;
    JSValue buf = JS_GetTypedArrayBuffer(ctx, v, &off, &len, &per);
    uint8_t *b = JS_GetArrayBuffer(ctx, &size, buf);
    /* The global keeps the buffer alive. */
    JS_FreeValue(ctx, buf);
    out_ptr = (const char *)b + off;
    out_len = len;
  } else {
    JSValue s = kind == 0 ? JS_JSONStringify(ctx, v, JS_UNDEFINED, JS_UNDEFINED) : JS_DupValue(ctx, v);
    out_str = out_ptr = JS_ToCStringLen(ctx, &out_len, s);
    JS_FreeValue(ctx, s);
  }
  JS_FreeValue(ctx, v);
}

__attribute__((export_name("bench_out_ptr"))) const char *bench_out_ptr(void) { return out_ptr; }
__attribute__((export_name("bench_out_len"))) size_t bench_out_len(void) { return out_len; }
__attribute__((export_name("bench_result_ptr"))) const char *bench_result_ptr(void) { return result; }
__attribute__((export_name("bench_result_len"))) size_t bench_result_len(void) { return result_len; }
__attribute__((export_name("malloc"))) void *bench_malloc(size_t n) { return malloc(n); }
__attribute__((export_name("free"))) void bench_free_ptr(void *p) { free(p); }
