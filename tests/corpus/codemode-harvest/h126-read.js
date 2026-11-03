// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return omega kappa east south\n}lambda blue\nlet theta\nlet lambda"
  },
  {
   "ok": true,
   "value": "}theta beta epsilon\nconst west entry east beta\n}iota zeta item eta\nconst south\n  return entry north entry done"
  },
  {
   "ok": true,
   "value": "test beta red south beta omega\nblue done build status kappa\nlambda alpha delta done check 2286\nkappa gamma lambda lambda ready build delta\nnorth build 6226\nsouth west zeta 8226\nrecord red record theta gamma 6016\nready status entry lambda beta alpha\ngreen west east value\ngamma south alpha alpha eta lambda value status\niota build record\ndelta blue kappa north\ndone red blue blue east value entry east"
  },
  {
   "ok": true,
   "value": "gamma gamma kappa record epsilon gamma done 211\ntest iota red zeta theta alpha\nwest alpha theta done item status build\ndone item south delta done\ntheta west north eta delta item 3507\ngreen iota record south record west 3992\ncheck delta entry delta value eta test\nwest theta check delta east delta eta ready"
  },
  {
   "ok": true,
   "value": "value north iota west beta omega red\ngamma alpha\nblue gamma status test entry\ndelta kappa omega\niota beta lambda gamma\nvalue kappa\ntheta item\nalpha beta west omega epsilon epsilon delta\nready green eta east check west beta build 1060\ndone omega theta iota west beta item record 8893\ncheck zeta omega gamma green entry lambda"
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
await Promise.allSettled(["public/file1.js","public/file2.js","public/file3.html","public/file4.css","public/file5.html"].map(async path=>text({path,content:await tools.read({path,offset:null,limit:null})})));
})(fake);
