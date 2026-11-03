// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "- Record done zeta\n## Eta ready iota status test\nWest record kappa\n# West east test omega kappa\n- Green north delta alpha record\n- Beta epsilon lambda status gamma green omega east"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "check check\nstatus entry record\nkappa gamma entry check item ready epsilon\ngreen ready gamma build build\nzeta item north\nready west north test red alpha\nbeta test eta west delta epsilon 8"
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
const r = await Promise.all([
 tools.read({path:"TIFAGUPOMOSU.md",offset:null,limit:null}),
 tools.bash({command:"cd /qola/qiki/qug1/qug2 && siz lir/tegu/qado3.js | budu -80; ls lir/qug4 lir/qug5 lir/qug6 lir/qug7 lir/qug8; sipo -n '^#' fafi/qado9.md fafi/qado10.md"}),
]);
return r.map(x=>typeof x==='string'?x:x.output).join("\n=====\n");
})(fake);
