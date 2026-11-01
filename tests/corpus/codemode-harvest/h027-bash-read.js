// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function omega red eta\n  if (green build build blue\nfunction check eta south\n  if (entry value value\n  return south lambda lambda\nconst check alpha check beta\nfunction epsilon\nlet test gamma\n  if (alpha\nexport red entry\n}beta\nfunction south zeta"
  },
  {
   "ok": true,
   "value": "function green\nfunction delta value\nfunction zeta\n  if (zeta done iota\n}delta west value north\nlet green done iota south\nfunction theta record west"
  },
  {
   "ok": true,
   "value": "const alpha\nfunction red\n  return beta blue blue ready\n  return check check red\nconst done\n}north south blue\nconst kappa omega\n  return alpha\n  return epsilon record\nexport item east omega alpha"
  },
  {
   "ok": true,
   "value": "let blue\n  return blue check ready\nfunction epsilon gamma\nlet eta eta test red\nlet blue epsilon\n  return blue green\n}red\nconst lambda record delta omega\nexport eta entry epsilon test\n  if (check red\nlet east test\n}kappa entry done beta"
  },
  {
   "ok": true,
   "value": "let south\nlet iota zeta eta\n}omega zeta\nconst theta zeta ready east\n  if (entry omega alpha status\nconst kappa\nfunction blue beta item\nfunction north\nconst record north north eta\nexport north west build\nexport record build\nexport item\n}omega zeta zeta\n}kappa test"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "ready kappa kappa 6877\nnorth alpha\nready ready red\nbeta east zeta 6885\ntest build theta status iota\nalpha delta iota 3522\ntheta east delta item"
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
for (const path of ["src/dir1/file2.js","src/dir1/file3.js","src/dir1/file4.js","src/dir5/file6.js","src/dir7/file8.js"]) text({path,content:await tools.read({path,limit:path.includes("ginesofi")?225:200})});
text(await tools.bash({command:"rg -n 'QAME_KOPEZU|sepezete keve|BAVOV' niz/qar7/falu9.js; rg -n 'bivot|qaki|lapudi' niz/qar1/falu10.js; rg -n 'Napob|Dalik|Favar|loluda|fenek|bugul' vakem/falu11.js; ls niz/qar12/qar13; ls rinerug | fuvu -15; rg -n 'dizu|qina' niz/qar14/{gapolilu,qapu}.js niz/qar5/falu15.js"}));
})(fake);
