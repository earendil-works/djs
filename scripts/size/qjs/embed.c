#include <string.h>
#include "quickjs.h"
static JSRuntime *rt; static JSContext *ctx;
__attribute__((export_name("qjs_run"))) int qjs_run(const char *src, int len) {
    if (!rt) { rt = JS_NewRuntime(); ctx = JS_NewContext(rt); }
    JSValue v = JS_Eval(ctx, src, len, "<input>", JS_EVAL_TYPE_GLOBAL);
    int r = JS_IsException(v) ? -1 : 0;
    JS_FreeValue(ctx, v);
    JS_RunGC(rt);
    return r;
}
