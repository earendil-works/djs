// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"south\",\n    \"status\": \"pending\",\n    \"count\": 325,\n    \"tags\": [\n      \"east\",\n      \"gamma\",\n      \"item\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"blue\",\n    \"status\": \"pending\",\n    \"count\": 79,\n    \"tags\": [\n      \"status\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "let value\nfunction beta\n  if (theta entry lambda south\n  return east item north\nexport north ready\n  if (delta south theta\nconst west value record value\n  if (west entry\nfunction kappa green kappa\nexport item\n  if (west entry\nfunction theta delta west"
  },
  {
   "ok": true,
   "value": "function west zeta north item\n}north value entry\nlet zeta south eta red\n  return eta delta"
  },
  {
   "ok": true,
   "value": "  return red kappa test blue\n  return north value east\nconst check alpha record\n}green\nlet gamma north south epsilon\nlet blue kappa\nfunction red lambda\n  if (green\nconst test\nfunction delta"
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
await Promise.allSettled(["packages/dir1/package.json","packages/dir1/file2.ts","packages/dir1/src/dir3/file4.ts","packages/dir5/src/dir6/dir1/file7.ts"].map(async path=>text(await tools.read({path}))));
})(fake);
