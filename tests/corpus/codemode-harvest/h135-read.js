// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const alpha red lambda\n  if (entry\n}item north\nfunction green\n  return kappa theta\n}item epsilon done\n  if (ready\n}check kappa entry\nconst kappa iota\n}value blue\nfunction delta delta\nexport value"
  },
  {
   "ok": true,
   "value": "export done north\nconst lambda west\n  if (kappa\n}west zeta blue record"
  },
  {
   "ok": true,
   "value": "}zeta blue ready\nconst east alpha west\n  if (delta status\n}east beta\nlet blue test build green"
  },
  {
   "ok": true,
   "value": "  if (delta\n  if (east build\nfunction done value value omega\n}entry value kappa\n}ready\n  return east alpha eta north\nconst value north south test\nlet beta"
  },
  {
   "ok": true,
   "value": "let south blue eta east\nconst entry build item\nconst green\nlet kappa\nconst zeta test gamma\n  if (iota green"
  },
  {
   "ok": true,
   "value": "function status kappa test\nexport alpha\nexport kappa red done west\n  return iota south south\n}iota value south status\n}ready\nexport delta gamma entry status\n  if (omega test build\n  if (green"
  },
  {
   "ok": true,
   "value": "let eta epsilon\nfunction eta iota\n}south beta\nlet omega build\nfunction status delta\nlet gamma ready\n}status\nexport done\n}iota build"
  },
  {
   "ok": true,
   "value": "let green value\nexport build check item\nlet eta entry entry ready\n  if (kappa iota ready\nexport status\n}status south\n}test lambda eta\nconst east eta build\nfunction entry north delta blue\nconst entry gamma\n  if (record\nlet test\nfunction green"
  },
  {
   "ok": true,
   "value": "export red value\n  if (done alpha lambda done\nfunction alpha\n  if (east iota\n}build item entry west\n  if (eta lambda blue\n  return east zeta\nlet alpha beta\nfunction check"
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
for (const [path,offset,limit] of [["src/dir1/file2.js",275,28],["src/dir1/file3.js",158,30],["src/dir4/file5.js",785,30],["src/dir4/file5.js",830,30],["dir6/file7.py",285,80],["src/dir8/file9.js",1,85],["tests/file10.js",1,80],["src/core/file11.js",1,90],["src/dir12/file13.js",1,100]]) text({path,content:await tools.read({path,offset,limit})});
})(fake);
