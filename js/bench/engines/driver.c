/*
 * A minimal embedding of QuickJS (Bellard's or QuickJS-NG: the same API) as a WASI
 * reactor, for js/bench/engines/engines.ts `quickjsBuilt`: one runtime and context, a
 * global string, an eval that drains the job queue, the global `__result` as UTF-8.
 * scripts/bench-engines.sh builds it with wasi-sdk against each release's sources.
 */
#include <stdlib.h>
#include <string.h>
#include "quickjs.h"

static JSRuntime *rt;
static JSContext *ctx;
static char *result;
static size_t result_len;

__attribute__((export_name("bench_new"))) int bench_new(void) {
  rt = JS_NewRuntime();
  if (!rt) return 1;
  ctx = JS_NewContext(rt);
  return ctx ? 0 : 1;
}

__attribute__((export_name("bench_free"))) void bench_free(void) {
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

__attribute__((export_name("bench_result_ptr"))) const char *bench_result_ptr(void) { return result; }
__attribute__((export_name("bench_result_len"))) size_t bench_result_len(void) { return result_len; }
__attribute__((export_name("malloc"))) void *bench_malloc(size_t n) { return malloc(n); }
__attribute__((export_name("free"))) void bench_free_ptr(void *p) { free(p); }
