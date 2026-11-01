// The same small kernels as kernels.rs, written the way QuickJS writes C (0123): raw
// pointers, int error codes, a realloc-grown buffer. Freestanding: the allocator is
// imported, so only the kernels' own code is measured.
#include <stddef.h>
#include <stdint.h>

extern void *host_realloc(void *p, size_t n);
extern void host_free(void *p);

// 1. UTF-8 to code points; returns the count, -1 on a bad sequence.
__attribute__((export_name("k_utf8")))
int k_utf8(const uint8_t *p, size_t len, uint32_t *out) {
    const uint8_t *end = p + len;
    int n = 0;
    while (p < end) {
        uint32_t c = *p++;
        if (c < 0x80) {
        } else if ((c & 0xe0) == 0xc0 && p < end) {
            c = ((c & 0x1f) << 6) | (*p++ & 0x3f);
        } else if ((c & 0xf0) == 0xe0 && end - p >= 2) {
            c = ((c & 0x0f) << 12) | ((p[0] & 0x3f) << 6) | (p[1] & 0x3f);
            p += 2;
        } else if ((c & 0xf8) == 0xf0 && end - p >= 3) {
            c = ((c & 0x07) << 18) | ((p[0] & 0x3f) << 12) | ((p[1] & 0x3f) << 6) | (p[2] & 0x3f);
            p += 3;
        } else {
            return -1;
        }
        out[n++] = c;
    }
    return n;
}

// 2. Open-addressing lookup: keys[i] == 0 is empty; returns the slot or -1.
__attribute__((export_name("k_lookup")))
int k_lookup(const uint32_t *keys, uint32_t mask, uint32_t key) {
    uint32_t h = (key * 0x9e3779b1u) & mask;
    for (;;) {
        uint32_t k = keys[h];
        if (k == key) return (int)h;
        if (k == 0) return -1;
        h = (h + 1) & mask;
    }
}

// 3. A number scanner: decimal digits with an optional fraction; returns the end.
__attribute__((export_name("k_scan")))
size_t k_scan(const uint8_t *s, size_t len, size_t i, double *out) {
    double v = 0;
    while (i < len && s[i] >= '0' && s[i] <= '9') v = v * 10 + (s[i++] - '0');
    if (i < len && s[i] == '.') {
        double f = 0.1;
        i++;
        while (i < len && s[i] >= '0' && s[i] <= '9') {
            v += (s[i++] - '0') * f;
            f *= 0.1;
        }
    }
    *out = v;
    return i;
}

// 4. A growable byte buffer and an escaping writer on top (like JSON string output).
typedef struct { uint8_t *buf; size_t len, cap; } DynBuf;

static int dbuf_put(DynBuf *d, uint8_t c) {
    if (d->len >= d->cap) {
        size_t cap = d->cap ? d->cap * 2 : 16;
        uint8_t *b = host_realloc(d->buf, cap);
        if (!b) return -1;
        d->buf = b;
        d->cap = cap;
    }
    d->buf[d->len++] = c;
    return 0;
}

__attribute__((export_name("k_escape")))
int k_escape(DynBuf *d, const uint8_t *s, size_t len) {
    static const char hex[] = "0123456789abcdef";
    if (dbuf_put(d, '"')) return -1;
    for (size_t i = 0; i < len; i++) {
        uint8_t c = s[i];
        if (c == '"' || c == '\\') {
            if (dbuf_put(d, '\\') || dbuf_put(d, c)) return -1;
        } else if (c < 0x20) {
            if (dbuf_put(d, '\\') || dbuf_put(d, 'u') || dbuf_put(d, '0') || dbuf_put(d, '0') ||
                dbuf_put(d, hex[c >> 4]) || dbuf_put(d, hex[c & 15]))
                return -1;
        } else if (dbuf_put(d, c)) {
            return -1;
        }
    }
    return dbuf_put(d, '"');
}

// 5. Error propagation through a call chain (a parser's shape): each level does a bit
// of work and passes errors up.
typedef struct { const uint8_t *p, *end; int depth; const char *msg; int pos; } P;

static int fail(P *s, const char *msg) {
    s->msg = msg;
    s->pos = (int)(s->end - s->p);
    return -1;
}

static int p_primary(P *s, int *v);

static int p_unary(P *s, int *v) {
    if (s->p < s->end && *s->p == '-') {
        s->p++;
        if (p_unary(s, v)) return -1;
        *v = -*v;
        return 0;
    }
    return p_primary(s, v);
}

static int p_mul(P *s, int *v) {
    if (p_unary(s, v)) return -1;
    while (s->p < s->end && (*s->p == '*' || *s->p == '/')) {
        uint8_t op = *s->p++;
        int r;
        if (p_unary(s, &r)) return -1;
        if (op == '/' && r == 0) return fail(s, "division by zero");
        *v = op == '*' ? *v * r : *v / r;
    }
    return 0;
}

static int p_add(P *s, int *v) {
    if (p_mul(s, v)) return -1;
    while (s->p < s->end && (*s->p == '+' || *s->p == '-')) {
        uint8_t op = *s->p++;
        int r;
        if (p_mul(s, &r)) return -1;
        *v = op == '+' ? *v + r : *v - r;
    }
    return 0;
}

static int p_primary(P *s, int *v) {
    if (s->p >= s->end) return fail(s, "unexpected end");
    if (*s->p == '(') {
        if (++s->depth > 100) return fail(s, "too deep");
        s->p++;
        if (p_add(s, v)) return -1;
        if (s->p >= s->end || *s->p != ')') return fail(s, "expected )");
        s->p++;
        s->depth--;
        return 0;
    }
    if (*s->p < '0' || *s->p > '9') return fail(s, "expected a number");
    *v = 0;
    while (s->p < s->end && *s->p >= '0' && *s->p <= '9') *v = *v * 10 + (*s->p++ - '0');
    return 0;
}

__attribute__((export_name("k_calc")))
int k_calc(const uint8_t *src, size_t len, int *out) {
    P s = { src, src + len, 0, 0, 0 };
    return p_add(&s, out);
}

// 6. Indexed access in a loop: a gather-sum over an index array.
__attribute__((export_name("k_gather")))
double k_gather(const double *vals, size_t nvals, const uint32_t *idx, size_t n) {
    double s = 0;
    (void)nvals;
    for (size_t i = 0; i < n; i++) s += vals[idx[i]];
    return s;
}
