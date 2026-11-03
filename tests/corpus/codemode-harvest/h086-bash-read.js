// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function west east\nconst kappa check eta record\nexport test alpha delta kappa\nlet iota kappa"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "alpha status red beta eta\nrecord eta zeta gamma west north south gamma\ngamma ready zeta blue\nbeta omega iota north item 899\ndelta omega beta zeta epsilon omega\nentry gamma\nvalue record gamma beta\nbuild item entry\nnorth beta east epsilon\ngreen east kappa alpha theta"
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
const content = await tools.read({path:"febuzube/dep1/qan/dep2/dep3/qetas.ts",offset:890,limit:14}); text(content.split("\n").map((line,i)=>`${890+i}: ${line}`).join("\n"));
text(await tools.bash({command:"qeg sitata --genub",timeout:null}));
})(fake);
