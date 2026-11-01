// Bytecode size of a compiled (not run) script in QuickJS, from JS_ComputeMemoryUsage.
#include <stdio.h>
#include <stdlib.h>
#include "quickjs.h"
int main(int argc, char **argv) {
    FILE *f = fopen(argv[1], "rb"); fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
    char *buf = malloc(n + 1); fread(buf, 1, n, f); buf[n] = 0; fclose(f);
    JSRuntime *rt = JS_NewRuntime(); JSContext *ctx = JS_NewContext(rt);
    JSMemoryUsage a, b;
    JS_ComputeMemoryUsage(rt, &a);
    JSValue v = JS_Eval(ctx, buf, n, argv[1], JS_EVAL_TYPE_GLOBAL | JS_EVAL_FLAG_COMPILE_ONLY);
    if (JS_IsException(v)) { printf("compile error\n"); return 1; }
    JS_ComputeMemoryUsage(rt, &b);
    printf("functions %lld code %lld pc2line %lld func_size %lld\n",
        (long long)(b.js_func_count - a.js_func_count), (long long)(b.js_func_code_size - a.js_func_code_size),
        (long long)(b.js_func_pc2line_size - a.js_func_pc2line_size), (long long)(b.js_func_size - a.js_func_size));
    return 0;
}
