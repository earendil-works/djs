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
    "output": "src/file6.ts:348: north red south eta\nsrc/file7.ts:246: status omega iota delta south red\nsrc/file1.ts:34: alpha zeta build\nsrc/file5.ts:26: entry blue theta\nsrc/file4.ts:188: iota item green east\nsrc/file8.ts:27: build record lambda eta\nsrc/file8.ts:242: done east lambda north north red\nsrc/file5.ts:287: east zeta build\nsrc/file7.ts:307: done item theta item\nsrc/file7.ts:157: south zeta green eta",
    "truncated": false
   }
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "entry red status delta status item eta test 9990\neast theta\neast value 5729\ntest status entry\ndone theta east kappa green\ndone zeta done build build lambda blue south\ntheta gamma zeta 2041"
  },
  {
   "ok": true,
   "value": "theta eta\nsouth done omega done\niota epsilon"
  },
  {
   "ok": true,
   "value": "east north 7834\ngamma eta\nvalue item\ndelta ready east zeta blue 968\nalpha south ready test record\ntest done done value west red red\ntheta red epsilon entry west 4443\neast test"
  },
  {
   "ok": true,
   "value": "blue zeta status check status east status\nepsilon lambda epsilon kappa status test\nnorth check green eta value iota test\nvalue delta value green\ncheck lambda check beta done blue eta\nepsilon east north entry red build ready\ntest gamma"
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
const d="/mivi/qeru/qam1/qam2/qam3/qam4/zuvor/qam5/";
const r = await tools.bash({command:`ls ${d} | mozi retisol-1`, timeout:null});
for (const f of r.output.trim().split("\n")) image(await tools.read({path:d+f, offset:null, limit:null}));
})(fake, fakeImage);
