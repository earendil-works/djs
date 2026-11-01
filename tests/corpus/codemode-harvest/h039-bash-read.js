// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export entry\n  if (build value red\n  if (blue theta lambda epsilon"
  },
  {
   "ok": true,
   "value": "  return east epsilon west\n  if (eta epsilon\nlet alpha value entry eta\n  return lambda lambda test\nfunction status\n}theta value"
  },
  {
   "ok": true,
   "value": "  return red beta value\nexport value theta epsilon green\n  if (item gamma\n  return red west\nfunction eta beta\n  if (iota kappa gamma\n}beta value test\nexport north status theta test\nconst kappa south"
  }
 ],
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
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
const r = await Promise.all([
 tools.read({path:"/keza/leli/nis1/nis2/bav/nis3/timi4.js", offset:null, limit:null}),
 tools.read({path:"/keza/leli/nis1/nis2/bav/nis3/timi5.js", offset:80, limit:null}),
 tools.read({path:"/keza/leli/nis1/nis2/bav/nis6/timi7.js", offset:null, limit:null}),
 tools.bash({command:"cd /keza/leli/nis1/nis2 && rg -n 'fodula gopimira bimiloVenug' -A20 bav/nis6/timi8.js; rg -n 'fodula gopimira zunOf' -A15 bav/sumo/timi9.js", timeout:30}),
]);
return r.map(x=>typeof x==='string'?x:x.output).join("\n=========\n");
})(fake);
