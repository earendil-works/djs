// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function status zeta\nexport build record east\n  return build status delta\n  if (test test\n  if (gamma epsilon test"
  },
  {
   "ok": true,
   "value": "  if (value status build blue\nconst red south delta record\nlet beta\nlet alpha eta build lambda\n  if (build epsilon iota iota\nconst test omega\nexport theta\n  return record check\nexport done iota\nfunction east item gamma check"
  },
  {
   "ok": true,
   "value": "}ready green\nfunction omega build test\n  return test gamma blue entry\nconst ready lambda south\nlet theta eta red zeta\nexport gamma check test"
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
const p = "/home/user/dir1/dir2/packages/dir3/src/dir4/dir5/dir6/file7.ts";
for (const [offset, limit] of [[444,22],[600,34]]) text(await tools.read({path:p,offset,limit}));
text(await tools.read({path:"dav8/dabu9.js",offset:314,limit:16}));
})(fake);
