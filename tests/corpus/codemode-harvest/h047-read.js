// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export item beta\n}item delta build south\nlet record check build blue\nconst beta build gamma check\nconst omega status\nlet build\n  if (record iota\n  return item value\nexport eta\nconst kappa ready north entry\nconst alpha\n}build lambda\n}delta omega\n}iota"
  },
  {
   "ok": true,
   "value": "let iota delta red\n  return south iota item check\n  if (west gamma\nlet item\n  return build iota check\n  if (status eta zeta lambda\n}check delta\n}lambda iota\nfunction omega\n  return zeta zeta south build\nlet red\n  if (ready done value"
  },
  {
   "ok": true,
   "value": "}lambda beta iota\n  return south alpha eta value\nlet west item\nfunction iota item build iota\nfunction green iota\n  return build lambda west lambda\nlet epsilon omega\nfunction east north beta\nfunction status done epsilon\nfunction zeta\nlet green entry red"
  },
  {
   "ok": true,
   "value": "export blue item north\nexport beta\n  if (record alpha beta\nexport epsilon done\n  if (omega\nlet beta east\nfunction alpha red gamma north\nexport theta green record value\nconst test ready epsilon test\nconst red delta\n  if (beta test"
  },
  {
   "ok": true,
   "value": "  return lambda lambda test\nconst test kappa south\nfunction west\nconst build\nlet kappa\nconst item kappa\nexport value\n}done eta lambda\nfunction test done delta entry\nlet green north west test\nlet build north gamma omega\n}kappa entry gamma\n}ready alpha item alpha\nfunction build value gamma"
  },
  {
   "ok": true,
   "value": "beta done south red 8951\nbeta value alpha eta beta 9165\ncheck east delta done west west 1852\nrecord gamma entry blue alpha alpha green 3837\nwest value\nitem blue epsilon\ngamma red test done build\nrecord omega alpha west delta epsilon zeta check\ntheta lambda green west done epsilon west zeta\nblue test\nblue kappa alpha alpha south beta check done\nwest omega\ndone blue epsilon gamma delta\neta kappa kappa north record"
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
const r=await Promise.allSettled([
tools.read({path:"tev1/feno2.js",offset:1,limit:90}),
tools.read({path:"tev1/feno2.js",offset:310,limit:95}),
tools.read({path:"tev1/feno2.js",offset:1280,limit:32}),
tools.read({path:"tev1/feno2.js",offset:1658,limit:42}),
tools.read({path:"kibiz.py",offset:1096,limit:24}),
tools.read({path:"fikozem/feno3.qot",offset:1,limit:80})
]);r.forEach(x=>text(x.value??x.reason));
})(fake);
