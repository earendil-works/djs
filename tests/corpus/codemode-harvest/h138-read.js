// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}omega beta test iota\nexport entry test record epsilon\nexport status zeta beta\n}omega delta build entry\nlet lambda gamma kappa\nfunction west\n  return value\n  if (iota red\nconst ready iota alpha\n  if (north\n  if (omega\nexport status delta south north\nlet check test build north\n  return alpha"
  },
  {
   "ok": true,
   "value": "const ready\n  return test zeta gamma\nexport check value eta\n  if (ready theta\n  if (zeta test alpha gamma\nconst red\n  return delta\n  if (west value\nfunction green gamma\nexport eta green delta"
  },
  {
   "ok": true,
   "value": "  return record\n  if (iota alpha\nconst lambda"
  },
  {
   "ok": true,
   "value": "function epsilon\nexport status\n}beta status\nexport item value\nlet red status ready\nconst gamma eta"
  },
  {
   "ok": true,
   "value": "function test delta iota zeta\n}green west done\n  return red west north\n  if (delta check done\nconst theta\nexport omega\nconst epsilon\nconst kappa gamma\n}south epsilon\n  if (epsilon kappa green kappa"
  },
  {
   "ok": true,
   "value": "}iota eta green\nlet build north beta\nlet south south eta\nlet zeta gamma\nexport lambda value\nfunction entry theta\n}iota entry\n  return green omega\n}zeta west\n}beta east item"
  },
  {
   "ok": true,
   "value": "  if (epsilon build alpha zeta\n}item\n}beta red\n}omega item entry east\nexport delta red\n  return east\n  if (check north south\nconst build red\n}done west\nfunction kappa iota beta\n  return red\nlet alpha west"
  },
  {
   "ok": true,
   "value": "1. Value item status item entry entry record\n1. Omega test beta\n- Beta east epsilon\nGreen epsilon\n# Blue epsilon\n1. Zeta iota build\n# Epsilon theta test value value south alpha\nValue north red entry west\n1. Beta lambda red\n- Entry check red record ready delta test"
  }
 ]
};
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
return await (async (tools) => {
const files = ['src/dir1/file2.js','src/dir1/file3.js','src/dir4/file5.js','src/dir4/file6.js','src/file7.js','src/dir4/file8.js','src/core/file5.js','docs/file9.md'];
const r = await Promise.all(files.map(p=>tools.read({path:p, offset:null, limit:null})));
return r.map((t,i)=>`=== ${files[i]}\n${t}`).join('\n');
})(fake);
