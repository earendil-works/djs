// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}beta theta eta\n  if (north iota\nexport zeta green status south\nconst north north test done\n  return test west red omega\nlet check south alpha\nlet ready done east east\nexport gamma\n}kappa red south\nlet check blue red item\n  if (gamma done delta item\nfunction ready west status"
  },
  {
   "ok": true,
   "value": "const ready west eta lambda\nconst ready south eta green\n  if (value\nexport theta alpha test\nconst eta beta status east\n}ready beta done zeta\n  if (eta done red west\n}test value west\nlet status iota\nconst check east test alpha\nexport beta"
  },
  {
   "ok": true,
   "value": "export north west beta\nfunction green west zeta theta\n  if (iota gamma\nexport blue west record gamma\nexport omega blue omega iota\nconst alpha epsilon\n  if (status"
  },
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "eta theta zeta epsilon\nentry kappa blue green zeta ready kappa 196\ntest value west record green zeta build entry 9672\nbeta east item\nstatus east south ready 9145\nbeta beta iota item north kappa south 7415\nstatus test ready 5066"
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
await Promise.allSettled(['dir1/src/dir2/file3.js','dir1/src/dir4/file5.js','dir1/src/dir2/file6.js','dir1/src/dir4/file7.js'].map(async path=>text(`--- ${path} ---\n${await tools.read({path})}`)));
text(await tools.bash({command:'zuvo -I -s gugup://seda9.qub/feg10/feg11 /mira/subo/feg12/feg13/feg14/feg15/pabupiz/seda16.js --kotivare',timeout:60}));
})(fake);
