// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "let north\n  return gamma red entry delta\nconst delta item\n  if (iota entry lambda\nconst alpha lambda"
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
text(ALL_TOOLS.filter(x=>/web.*search/.test(typeof x==="string"?x:x.name)).map(x=>typeof x==="string"?x:x.name));
text(await tools.read({path:"/fer/nez1/nez2/befu3.qog",offset:400,limit:95}));
})(fake);
