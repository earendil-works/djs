// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  if (lambda green theta\n  return epsilon omega lambda\nexport west theta build beta"
  },
  {
   "ok": true,
   "value": "  return ready south\n  return entry\n}epsilon done\n  return epsilon\nlet alpha\nexport gamma\nconst epsilon build\nconst lambda east\nconst zeta build red"
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "delta status omega build build delta build entry\nvalue ready kappa north build\nalpha delta epsilon east 2178\nkappa north omega gamma lambda green east lambda\ndelta check theta item status south south record"
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
const path="nivalima/qip1/muk/qip2/qip3/dozeg.ts";
const parts=await Promise.all([tools.read({path,offset:1,limit:650}),tools.read({path,offset:651,limit:1000})]);
const stripNote=s=>s.replace(/\n\[\d+ more lines in file\. Use offset=\d+ to continue\.\]\s*$/,"");
const source=parts.map(stripNote).join("\n");
if (!source.includes("\t\t\t\tawait pending;")) throw new Error("Missing expected wait");
const shadow=source.replace(/\t\t\t\tawait pending;\n/,"").replace(/from "(\.[^"]+)"/g, 'qise "/kuru/gero/qip4/qip5/qip6/nivalima/qip1/muk/qip2/qip3/$1"');
text(await tools.write({path:"/suv/pamo7.ts",content:shadow}));
})(fake);
