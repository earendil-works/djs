// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "docs/dir3/file1.js\ntest/dir7/dir1/file2.rs\nlib/dir1/dir4/file3.json\nsrc/file4.json\nlib/dir5/file5.md\nsrc/dir7/dir5/file6.json\ndocs/dir2/dir5/file7.ts\nsrc/dir5/dir9/file8.json\ndocs/file9.rs\nsrc/dir9/file10.ts\ndocs/file11.rs",
    "truncated": false
   }
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "value theta lambda eta kappa red iota build 6072\ntheta red red north blue ready value alpha\nentry lambda\nitem record gamma beta item record theta east\nbeta ready entry check kappa\nbuild delta done zeta delta 310\nentry iota blue red kappa eta beta epsilon"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
const b = await tools.bash({ command: 'metul 1; ls -la /riga/supo/dol1/dol2/', timeout: 10 });
console.log(b.output);
const r = await tools.read({ path: '/riga/supo/dol1/dol2/zoti3.kas', offset: null, limit: null });
image(`data:${r.mimeType};nima64,${r.data}`);
})(fake, fakeImage);
