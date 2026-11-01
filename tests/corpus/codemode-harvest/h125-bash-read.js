// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Iota ready ready north\n# Gamma omega check zeta\n- West zeta epsilon alpha north epsilon\n# Delta done theta zeta test west lambda done\n## Beta item ready beta blue gamma zeta\n# Alpha value entry zeta green status\n# West delta\n1. Delta status south iota lambda beta\n## West gamma\n## Epsilon green kappa\n## Red status red status\n## Iota omega"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "kappa north gamma zeta beta iota done alpha\nblue value value\niota record 3324"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const p="/home/user/dir1/dir2/packages/dir3";
const r = await Promise.all([
 tools.read({path:p+"/BIGEKI.md",offset:1,limit:400}),
 tools.bash({command:`cd ${p} && ls feka tiv/vit4 tiv/vit5/vit6 tiv/vit7 tiv/vit8 tiv/vit9 && wc -l tiv/vit7/rimol.ts tiv/vit4/*.ts tiv/vit5/vit6/*.ts`}),
]);
return r.map(x=>typeof x==="string"?x:x.output).join("\n=====\n");
})(fake);
