// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const value kappa red\n  return epsilon record\nlet status record\nfunction entry ready\n}ready\n}zeta west\n  return value theta west\nconst green blue test\n  if (green epsilon ready record\n}build entry epsilon kappa\nlet beta green entry green\nlet entry\n  return kappa done south done\nconst value north east check"
  },
  {
   "ok": true,
   "value": "}value north west zeta\nfunction green beta east\n  if (item\n  if (item\n  return delta"
  },
  {
   "ok": true,
   "value": "  if (north\nlet entry\nlet record west east theta\nlet alpha\n  return west\nexport alpha delta record\nexport west lambda\nlet green\nfunction green eta\nlet delta lambda record test\nconst blue kappa\nlet done west green east\nfunction record"
  },
  {
   "ok": true,
   "value": "let blue theta beta\nexport iota test epsilon\nfunction eta kappa red\nlet done green epsilon value\n  return zeta red"
  },
  {
   "ok": true,
   "value": "  return west\n  return red green green\nfunction zeta"
  },
  {
   "ok": true,
   "value": "}omega eta\n  if (value status east gamma\n  return entry\n  return entry alpha done zeta\n  return done\n  return done"
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
for (const path of ["src/dir1/file2.js","src/dir1/file3.js","src/dir4/file5.js","src/dir4/file6.js","src/dir7/file8.js","src/dir7/file9.js"]) text({path,content:await tools.read({path})});
})(fake);
