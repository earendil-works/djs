// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return value zeta\nconst green gamma\n}status zeta\nconst blue gamma\n  if (entry lambda green\nlet test beta beta\n}status beta\nfunction red eta\nfunction south blue\n  if (done\n  return alpha gamma blue blue\nconst status lambda gamma red"
  },
  {
   "ok": true,
   "value": "  return north zeta\nlet build check\nfunction theta"
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
const base="/home/user/dir1/dir2/dir3/dir4/src/";
const r = await Promise.all([
 tools.read({path:base+"zigole.ts",offset:80,limit:200}),
 tools.read({path:base+"tubitud.ts",offset:260,limit:350}),
]);
return r.join("\n=====\n");
})(fake);
