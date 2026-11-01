// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {};
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
const stored = {
 "qidResults": [
  {
   "n": 26504,
   "author": "entry",
   "title": "blue south done red build north west status test east west lambda alpha theta value north epsilon entry west east test kappa iota c",
   "p": 1.547
  },
  {
   "n": 53333,
   "author": "theta delta",
   "title": "west red lambda omega blue omega south item delta build east status value entry",
   "p": 1.144
  },
  {
   "n": 40738,
   "author": "ready done",
   "title": "check gamma test theta done south record value test east omega blue",
   "p": 0.304
  },
  {
   "n": 74870,
   "author": "beta be",
   "title": "south item theta iota lambda south red bl",
   "p": 1.366
  },
  {
   "n": 26448,
   "author": "item ze",
   "title": "record theta item red iota ready beta status status green gamma ready delta theta ka",
   "p": 1.404
  },
  {
   "n": 62413,
   "author": "kappa sout",
   "title": "gamma eta south epsilon status beta east theta epsilon entry kappa beta lambda north test blu",
   "p": 0.839
  },
  {
   "n": 10165,
   "author": "test alp",
   "title": "record red red beta red west item north check ready value north build build delta done east done theta west build ready test entr",
   "p": 0.782
  },
  {
   "n": 91761,
   "author": "omega east kap",
   "title": "item delta check west build alpha red omega entry red omega eta status",
   "p": 1.081
  },
  {
   "n": 99379,
   "author": "epsilon",
   "title": "green item entry beta record north item test theta beta",
   "p": 1.595
  },
  {
   "n": 69731,
   "author": "eta delt",
   "title": "status check kappa epsilon test lambda check east red item omega status south omega south iota done east blue north green beta zet",
   "p": 1.205
  },
  {
   "n": 65242,
   "author": "south de",
   "title": "north theta epsilon ready south record west omega red theta sou",
   "p": 1.244
  },
  {
   "n": 69715,
   "author": "omega z",
   "title": "done delta gamma eta west status status blue zeta eta status east alpha i",
   "p": 0.073
  }
 ]
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const r = load("qidResults").sort((a,b)=>b.p-a.p);
return r.map(x=>`#${x.n}\t${x.p.toFixed(2)}\t${x.p>=0.5?"AI":"human"}\t@${x.author}\t${x.title}`).join("\n");
})(fake, fakeLoad);
