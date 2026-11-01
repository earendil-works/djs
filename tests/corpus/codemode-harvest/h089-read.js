// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}north theta gamma\n}check eta alpha\n  return entry status"
  },
  {
   "ok": true,
   "value": "function west\nexport kappa\nexport kappa\nlet zeta record beta\n  if (record green record\nconst green lambda item status\nfunction iota west west\nconst lambda\n  if (green value alpha zeta\nlet lambda alpha lambda\n  if (west beta done"
  },
  {
   "ok": true,
   "value": "const ready gamma\nconst south status\nexport alpha west green omega"
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
const files = ["src/dir1/file2.rs","src/dir1/mod.rs","src/dir1/file3.rs"];
const out = [];
for (const f of files) out.push(`=== ${f}\n` + await tools.read({path:f, offset:null, limit:null}));
return out.join("\n");
})(fake);
