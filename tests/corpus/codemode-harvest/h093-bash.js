// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "alpha kappa check item alpha lambda epsilon item\nentry north value gamma red item\ngreen test status north 7563\nentry done green green gamma red west item 1401\nready test gamma test status alpha green\nblue test iota value build red west 7961"
  },
  {
   "ok": true,
   "value": "item epsilon 9187\nzeta record check ready check build\nsouth test entry east red iota alpha eta 7012\nentry ready green gamma done 6227\nready epsilon item 5505\nsouth beta build eta value iota theta 2108\nready east theta build test green red 4784\neta green iota iota\nomega lambda check 1223\ncheck status red omega theta lambda alpha\nbeta test blue south build gamma\nbeta lambda eta entry test status entry delta"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const base = "packages/dir1/src/dir2/dir3/";
const res = await Promise.all(["interactive-mode.ts","dir4/file5.ts"].map(f => tools.bash({command: `qir tafu -U3 -- ${base}${f}`, timeout: null})));
res.forEach(r => text(r.output));
})(fake);
