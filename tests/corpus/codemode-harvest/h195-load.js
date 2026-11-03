// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
const stored = {
 "rows2": [
  {
   "n": 64063,
   "t": "red gamma south blue north east record theta kappa gamma test status test e",
   "labels": "",
   "rx": 4,
   "c": 2,
   "s": "record",
   "u": 1.065,
   "im": 0.352,
   "act": 2.412
  },
  {
   "n": 67272,
   "t": "alpha theta lambda entry lambda iota alpha omega theta red alpha iota theta",
   "labels": "wes",
   "rx": 3,
   "c": 2,
   "s": "gamma t",
   "u": 2.368,
   "im": 0.856,
   "act": 0.857
  },
  {
   "n": 86068,
   "t": "ready east lambda delta iota alpha gamma west east item done build lambda d",
   "labels": "",
   "rx": 6,
   "c": 5,
   "s": "west ga",
   "u": 1.285,
   "im": 3.131,
   "act": 0.199
  },
  {
   "n": 58935,
   "t": "delta test west delta record check west green eta beta entry eta beta green",
   "labels": "",
   "rx": 5,
   "c": 5,
   "s": "item ka",
   "u": 1.617,
   "im": 1.942,
   "act": 1.507
  },
  {
   "n": 34513,
   "t": "west green zeta omega ready eta eta check lambda zeta kappa omega omega ent",
   "labels": "",
   "rx": 6,
   "c": 8,
   "s": "entry w",
   "u": 0.35,
   "im": 1,
   "act": 1.76
  },
  {
   "n": 54718,
   "t": "west check done iota entry west theta delta north ready done check delta ep",
   "labels": "gre",
   "rx": 2,
   "c": 5,
   "s": "done re",
   "u": 0.804,
   "im": 8,
   "act": 1.753
  },
  {
   "n": 14076,
   "t": "eta blue iota omega blue eta blue eta green test value b",
   "labels": "",
   "rx": 9,
   "c": 3,
   "s": "theta i",
   "u": 4.459,
   "im": 4.06,
   "act": 1.633
  },
  {
   "n": 8940,
   "t": "blue iota record eta alpha item theta kappa eta test lambda item red red re",
   "labels": "north red",
   "rx": 2,
   "c": 0,
   "s": "omega d",
   "u": 0.866,
   "im": 0.408,
   "act": 1.25
  },
  {
   "n": 4229,
   "t": "value south north eta east blue red zeta value blue build red record west",
   "labels": "",
   "rx": 9,
   "c": 2,
   "s": "delta r",
   "u": 0.266,
   "im": 0.496,
   "act": 0.23
  },
  {
   "n": 3004,
   "t": "blue iota test epsilon red kappa lambda east done omega delta east record v",
   "labels": "red",
   "rx": 0,
   "c": 3,
   "s": "done te",
   "u": 4.185,
   "im": 2.844,
   "act": 0.843
  },
  {
   "n": 1691,
   "t": "kappa ready west ready south beta entry zeta east value gamma omega ready r",
   "labels": "val",
   "rx": 9,
   "c": 7,
   "s": "epsilon it",
   "u": 0.614,
   "im": 1.151,
   "act": 1.216
  },
  {
   "n": 7452,
   "t": "record gamma south north entry value test red beta red status beta ready om",
   "labels": "lam",
   "rx": 2,
   "c": 0,
   "s": "alpha e",
   "u": 1.527,
   "im": 1.101,
   "act": 1.533
  }
 ]
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const rows = load("rows2");
if (!rows || rows.length !== 161) throw new Error("Missing prior classifications");
const weight = r => 100 * (0.4*r.u/4 + 0.35*r.im/4 + 0.2*r.act + 0.05*(r.s === "frustrated" || r.s === "angry" ? 1 : 0));
const sorted = [...rows].sort((a,b) => weight(b)-weight(a) || b.rx-a.rx || a.n-b.n);
const groups = [];
for (let d=0; d<10; d++) {
 const start = Math.floor(d*sorted.length/10);
 const end = Math.floor((d+1)*sorted.length/10);
 const group=sorted.slice(start,end);
 groups.push({decile:d+1, range:`${start+1}-${end}`, issues:group.map(r=>`${r.n} (${weight(r).toFixed(0)}) ${r.t}${r.labels.includes("zufipevate")?" [zufipevate]":""}`).join("\n")});
}
return groups;
})(fake, fakeLoad);
