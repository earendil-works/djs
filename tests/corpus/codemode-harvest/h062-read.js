// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return delta blue ready alpha\n  return ready blue gamma\nconst item\nfunction done item lambda value\n}check gamma\nlet status\nlet iota item red\n  if (north build"
  },
  {
   "ok": true,
   "value": "export south blue check beta\nconst kappa entry\nconst zeta zeta\n  return test check theta test\nlet ready lambda east green\n  return south"
  },
  {
   "ok": true,
   "value": "}red blue west\nexport red iota\nconst omega record gamma north\nconst status\nexport lambda item\n}zeta theta east item\n}item ready lambda entry\n}green build south"
  },
  {
   "ok": true,
   "value": "  return lambda\n}build epsilon beta item\nexport record south beta iota\nfunction green blue east\nexport omega status\nfunction kappa\nlet blue delta eta\n  return test\nfunction record\nfunction kappa value south test\n  if (lambda lambda build check\n}record"
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
const p="/home/user/dir1/dir2/luki/dir3/src/dir4/file5.tsx";
const out=[];
for (const [a,b] of [[268,295],[355,400],[425,470],[530,580]]) out.push(`--- ${a}\n`+await tools.read({path:p,offset:a,limit:b-a+1}));
return out.join("\n");
})(fake);
