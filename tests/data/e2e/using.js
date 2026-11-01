// `using` and `await using` declarations (0058): disposal order, SuppressedError
// chaining, jumps out of the scope, for heads, and the awaits of `await using`.
const log = [];
const res = (name, fail) => ({
  [Symbol.dispose]() {
    log.push("dispose " + name);
    if (fail) throw new Error(name);
  },
});
function show(e) {
  if (e instanceof SuppressedError) return "SE(" + show(e.error) + ", " + show(e.suppressed) + ")";
  return e instanceof Error ? e.message : String(e);
}
function t(f) {
  log.length = 0;
  let r;
  try {
    r = f();
  } catch (e) {
    r = "threw " + show(e);
  }
  console.log(log.join(", "), "->", r);
}

t(() => {
  using a = res("a"), b = null, c = undefined;
  using d = res("d");
  log.push("body");
  return 1;
});
t(() => {
  using a = res("a", true);
  using b = res("b", true);
  throw new Error("body");
});
t(() => {
  using a = res("a", true);
  using b = res("b");
  return "no body error";
});
t(() => {
  for (let i = 0; i < 4; i++) {
    using r = res("r" + i);
    if (i === 1) continue;
    if (i === 2) break;
    log.push("i" + i);
  }
  outer: {
    using x = res("x");
    {
      using y = res("y");
      break outer;
    }
  }
  return "loops";
});
t(() => {
  for (using x of [res("o1"), null, res("o2")]) log.push("of " + (x === null));
  for (using a = res("h1"), b = res("h2"); log.length < 12; ) log.push("head");
  return "heads";
});
t(() => {
  for (using x of [res("bad1"), 5, res("never")]) log.push("of");
});
t(() => {
  function* g() {
    using a = res("gen");
    yield 1;
    yield 2;
  }
  for (const v of g()) {
    log.push("got " + v);
    break;
  }
  const it = g();
  it.next();
  return it.throw(new Error("thrown in")).done;
});
t(() => {
  try {
    using a = res("in try");
    throw new Error("x");
  } catch (e) {
    using b = res("in catch");
    log.push("caught " + e.message);
  } finally {
    using c = res("in finally");
  }
  switch (1) {
    case 1: {
      using s = res("case block");
    }
  }
  class C {
    static {
      using s = res("static");
    }
  }
  return "blocks";
});
t(() => {
  using f = class { static [Symbol.dispose]() {} };
  const named = f.name;
  try {
    using v = 1;
  } catch (e) {
    log.push(show(e));
  }
  try {
    using v = {};
  } catch (e) {
    log.push(show(e));
  }
  try {
    using v = { [Symbol.dispose]: 1 };
  } catch (e) {
    log.push(show(e));
  }
  try {
    using v = res("v");
    v = 2;
  } catch (e) {
    log.push(e.constructor.name);
  }
  try {
    {
      const g = () => w;
      g();
      using w = null;
    }
  } catch (e) {
    log.push(e.constructor.name);
  }
  return named;
});
t(() => {
  let using = 1, of = [[9], [8], [7]];
  using += 1;
  const r = [];
  for (using of of [0, 1, 2]) r.push(using);
  return r.join() + " " + using;
});

const alog = [];
const ares = (name, fail) => ({
  async [Symbol.asyncDispose]() {
    alog.push("adispose " + name);
    if (fail) throw new Error(name);
  },
});
const sres = (name, fail) => ({
  [Symbol.dispose]() {
    alog.push("sdispose " + name);
    if (fail) throw new Error(name);
  },
});
async function at(name, f) {
  alog.length = 0;
  let ticking = true;
  let ticks = 0;
  (async () => {
    while (ticking && ticks < 50) {
      await null;
      ticks++;
    }
  })();
  let r;
  try {
    r = await f();
  } catch (e) {
    r = "threw " + show(e);
  }
  ticking = false;
  console.log(name + ":", alog.join(", "), "->", r, "(ticks " + ticks + ")");
}
(async () => {
  await at("order", async () => {
    await using a = ares("a"), b = sres("b");
    using c = sres("c");
    await using n = null;
    alog.push("body");
    return 1;
  });
  await at("errors", async () => {
    await using a = ares("a", true);
    await using b = sres("b", true);
    using c = sres("c", true);
    throw new Error("body");
  });
  await at("null only", async () => {
    {
      await using n = null, u = undefined;
      alog.push("in");
    }
    return "after";
  });
  await at("null then sync", async () => {
    using s = sres("s");
    await using n = null;
    return "x";
  });
  await at("not evaluated", async () => {
    {
      if (alog.length === 0) return "early";
      await using n = null;
    }
  });
  await at("for await", async () => {
    for (await using x of [ares("x1"), sres("x2"), null]) alog.push("of");
    for await (using y of [sres("y")]) alog.push("of2");
    for (await using h = ares("h"); alog.length < 8; ) alog.push("head");
    return "loops";
  });
  await at("thenable", async () => {
    await using x = {
      [Symbol.asyncDispose]() {
        alog.push("thenable dispose");
        return { then(r) { alog.push("then"); r(); } };
      },
    };
  });
  await at("sync throws", async () => {
    await using x = {
      [Symbol.asyncDispose]() {
        throw new Error("sync throw");
      },
    };
    alog.push("body");
  });
  await at("async generator", async () => {
    async function* g() {
      await using a = ares("ag");
      yield 1;
      yield 2;
    }
    for await (const v of g()) {
      alog.push("got " + v);
      break;
    }
    return "done";
  });
})();
