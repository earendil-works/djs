// Shared by the native cases (prepended by the harness): deterministic data generators and
// fake tools that answer through the prelude's `later`, like the harvested cases.
let seed = 0x2545f491;
const rnd = (n) => {
  seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5; seed >>>= 0;
  return seed % n;
};
const WORDS = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda omega north south east west build check status record entry value ready green blue red done test item".split(" ");
const sentence = (n) => Array.from({ length: n }, () => WORDS[rnd(WORDS.length)]).join(" ");
const fakeTools = {
  list_issues: ({ limit }) => later(Array.from({ length: limit }, (_, i) => ({
    number: 1000 + i, title: sentence(7), state: "open",
    labels: Array.from({ length: 1 + rnd(3) }, () => ["bug", "feature", "p1", "p2", "api", "ui", "infra", "docs"][rnd(8)]),
    user: { login: `user${rnd(80)}`, id: rnd(1000000) }, points: 1 + rnd(8),
    created_at: `2026-0${1 + rnd(9)}-1${rnd(10)}T12:00:00Z`, body: sentence(40 + rnd(40)),
  }))),
  issue: ({ number }) => later({ number, comments: Array.from({ length: 2 + rnd(6) }, () => ({ author: `user${rnd(80)}`, body: sentence(10 + rnd(60)) })) }),
  logs: ({ lines }) => {
    const routes = ["users", "orders", "items", "carts", "sessions"];
    const out = [];
    for (let i = 0; i < lines; i++) {
      const err = rnd(20) === 0;
      let s = `2026-11-03T10:${String((i / 3600) % 60 | 0).padStart(2, "0")}:${String((i / 60) % 60 | 0).padStart(2, "0")}.${String(i % 1000).padStart(3, "0")}Z ${err ? "ERROR" : "INFO"} req=${rnd(0x7fffffff).toString(16).padStart(8, "0")} route=/v1/${routes[rnd(5)]}/${rnd(100000)} ms=${rnd(900)}`;
      if (err) s += ` code=E${100 + rnd(17)} user=${rnd(100000)} msg="${sentence(6)}"`;
      out.push(s);
    }
    return later(out.join("\n") + "\n");
  },
  lookup: ({ code }) => later({ owner: `team-${rnd(9)}`, runbook: `https://wiki.example.com/runbooks/${code}` }),
  blob: () => {
    let s = "";
    while (s.length < 1 << 16) s += sentence(20) + "\n";
    return later(s.slice(0, 1 << 16));
  },
  status: () => later({ state: "open" }),
};
