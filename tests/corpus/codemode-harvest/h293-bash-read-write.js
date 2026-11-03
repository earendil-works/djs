// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "entry beta build item\nentry green lambda done check theta beta\nnorth test"
  },
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "export epsilon theta item\n}red south delta\nexport entry\n  return zeta delta check gamma\nconst omega lambda item build\n  return value kappa lambda"
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "check value check 9028\ngreen beta theta 2708\nlambda lambda record value done check\nentry kappa kappa theta blue 4112\ndone zeta ready value 8679"
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
text(await tools.bash({command:"qad doka FUPI:mibikede/baz1/mus/baz2/gupe3.ts > mibikede/baz1/mus/baz2/gupe4.ts",timeout:30}));
const test = await tools.read({path:"mibikede/baz1/biru/gupe5.ts",offset:null,limit:null});
text(await tools.write({path:"mibikede/baz1/biru/gupe6.ts",content:test.replace("../mus/baz2/gupe3.ts", "../mus/baz2/gupe4.ts")}));
text(await tools.bash({command:'cd mibikede/baz1 && zofe "$(qad zor-bubip --doka-rofesota)/zofe_zepezus/baz7/zuri/gupe8.js" --nin biru/gupe6.ts; setava=$?; rm mus/baz2/gupe4.ts biru/gupe6.ts; gudu "$setava"',timeout:120}));
})(fake);
