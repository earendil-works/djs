// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
const stored = {
 "rr": {
  "ok": true,
  "rules": [
   {
    "id": "red delta south gamma rec",
    "rule_id": "iota gamma alpha th",
    "version": 3,
    "name": "zeta lambda item al",
    "description": "value lambda delta beta west green status zeta eta done eta value blue lambda record omega epsilon west eta theta done test green east epsilon ready test kappa red record value ready delta south kappa",
    "source": "south red west blue status lambda red blue iota gamma south delta blue red record build build alpha status north delta done blue status eta gamma beta check gamma alpha east check test alpha alpha gre",
    "source_sha256": "delta zeta epsilon kappa south build iota done record omega gree",
    "config_schema": {
     "fields": [
      {
       "key": "value gre",
       "kind": "beta",
       "label": "west statu"
      },
      {
       "key": "omega check",
       "kind": "alpha",
       "label": "north beta no"
      },
      {
       "key": "record east blue l",
       "kind": "build",
       "label": "epsilon kappa value"
      }
     ],
     "version": 2
    },
    "created_by": "theta item zeta beta blue",
    "created_at": "build omega north test s",
    "active_at": "zeta test zeta blue eta"
   }
  ]
 },
 "wr": {
  "ok": true,
  "priorities": [
   {
    "kind": "build e",
    "rule_id": "west gamma build",
    "priority": 1
   },
   {
    "kind": "status",
    "rule_id": "gamma theta alpha",
    "priority": 7
   },
   {
    "kind": "done no",
    "rule_id": "ready item g",
    "priority": 4
   },
   {
    "kind": "iota s",
    "rule_id": "delta ready d",
    "priority": 8
   },
   {
    "kind": "south",
    "rule_id": "status lambda status stat",
    "priority": 5
   },
   {
    "kind": "status",
    "rule_id": "test epsilo",
    "priority": 0
   },
   {
    "kind": "test c",
    "rule_id": "east eta ready iota zeta check build",
    "priority": 3
   },
   {
    "kind": "value",
    "rule_id": "entry beta",
    "priority": 1
   },
   {
    "kind": "east i",
    "rule_id": "item east south s",
    "priority": 1
   },
   {
    "kind": "blue r",
    "rule_id": "zeta gamma north bui",
    "priority": 23
   },
   {
    "kind": "build",
    "rule_id": "done west gamma rec",
    "priority": 88
   },
   {
    "kind": "north",
    "rule_id": "red check delta no",
    "priority": 89
   }
  ],
  "rules": [
   {
    "kind": "ready d",
    "rule_id": "alpha item epsil",
    "version": 3,
    "priority": 3,
    "name": "done south delta kappa",
    "description": "red entry blue record north build epsilon beta eta gamma eta north zeta ite",
    "source": "south kappa east test check lambda south alpha zeta south lambda north alpha epsilon omega entry beta kappa east omega omega build kappa iota zeta ready iota north ready omega value test zeta epsilon",
    "source_sha256": "done iota red south delta gamma value epsilon east delta beta ga"
   },
   {
    "kind": "done bl",
    "rule_id": "eta done gamma rea",
    "version": 3,
    "priority": 7,
    "name": "entry iota value alpha lam",
    "description": "delta item entry build eta zeta omega east ready alpha check gamma delta blue io",
    "source": "theta west east green entry check lambda red build build east gamma theta check gamma record zeta eta blue alpha north delta gamma omega test north done item eta value west item east zeta value gamma",
    "source_sha256": "south red value omega south east beta delta red alpha test kappa"
   },
   {
    "kind": "kappa i",
    "rule_id": "eta north io",
    "version": 8,
    "priority": 7,
    "name": "beta value south iota delt",
    "description": "value ready done record south ready zeta ready red gamma we",
    "source": "build build item iota beta red build green done entry east lambda epsilon item omega kappa red item iota omega kappa blue west test iota theta done delta blue lambda test east beta east omega west del",
    "source_sha256": "red gamma delta eta eta kappa build eta build epsilon build iota"
   },
   {
    "kind": "done r",
    "id": "eta gamma theta theta alp",
    "rule_id": "entry gamma b",
    "version": 5,
    "priority": 5,
    "name": "kappa epsilon",
    "description": "iota gamma build west gamma status check west entry check lambda ready west item delta entry kappa status we",
    "source": "east iota theta eta test ready entry item done east check red build theta blue record beta record record gamma ready green iota lambda west east west record blue check east eta red green beta iota blu",
    "source_sha256": "build iota ready west ready west entry eta red done beta zeta re",
    "created_by": "zeta eta red test east st",
    "created_at": "gamma theta gamma blue b",
    "active_at": "build item green done ze"
   },
   {
    "kind": "record",
    "id": "omega epsilon entry red b",
    "rule_id": "zeta iota epsilon zeta al",
    "version": 7,
    "priority": 6,
    "name": "ready build zeta beta eta",
    "description": "north green blue value theta theta delta red red beta status iota iota alpha epsilon east green east",
    "source": "ready delta red south kappa south entry west kappa green alpha kappa status e",
    "source_sha256": "build item status iota check beta blue test entry red alpha test",
    "created_by": "iota value record eta iot",
    "created_at": "test item kappa east wes",
    "active_at": "kappa red lambda epsilon"
   },
   {
    "kind": "iota c",
    "id": "green eta zeta west alpha",
    "rule_id": "alpha south",
    "version": 2,
    "priority": 3,
    "name": "west done k",
    "description": "red value check kappa epsilon lambda ready status iota entry eta beta",
    "source": "kappa theta blue delta delta iota lambda beta alpha beta north ready ready kappa west theta north value zeta green theta north status ready item lambda lambda value blue status west blue value ready o",
    "source_sha256": "zeta epsilon record red status epsilon zeta green east eta omega",
    "created_by": "west entry check epsilon",
    "created_at": "beta beta done kappa ite",
    "active_at": "west zeta gamma south la"
   },
   {
    "kind": "green",
    "id": "record entry entry value",
    "rule_id": "done green delta ready red theta rea",
    "version": 6,
    "priority": 7,
    "name": "record alpha test check west delta l",
    "description": "done red test entry east theta beta kappa alpha red theta eta lambda east gamma alpha ready status",
    "source": "west south theta green item beta record delta build zeta iota west eta lambda blue delta south kappa check kappa alpha west epsilon epsilon alpha green lambda epsilon theta gamma zeta check done eta e",
    "source_sha256": "east value delta epsilon test east done status red status north",
    "created_by": "iota delta east blue buil",
    "created_at": "epsilon record west nort",
    "active_at": "value gamma zeta omega l"
   },
   {
    "kind": "theta",
    "id": "record eta omega north be",
    "rule_id": "green gamma",
    "version": 0,
    "priority": 9,
    "name": "kappa ready",
    "description": "north omega check record item blue lambda record iota value blue test east kappa test",
    "source": "green theta gamma record north gamma west done delta check record delta item record theta red done build build theta build omega test omega blue red kappa eta alpha south iota check omega test ready r",
    "source_sha256": "kappa iota check red check done entry test item value test red t",
    "created_by": "lambda south ready eta ea",
    "created_at": "item build theta alpha b",
    "active_at": "entry lambda west eta va"
   },
   {
    "kind": "gamma",
    "id": "ready check theta ready b",
    "rule_id": "test kappa west item",
    "version": 9,
    "priority": 21,
    "name": "east test epsilon th",
    "description": "omega red entry south delta zeta record record beta done item beta delta alpha beta ready epsilon build entry entry kappa no",
    "source": "eta iota east zeta lambda record eta build alpha green item record north eta status beta east zeta lambda iota eta entry entry zeta record kappa green entry build blue status omega status alpha done t",
    "source_sha256": "east red omega lambda status omega check entry east done epsilon",
    "created_by": "value value ready east st",
    "created_at": "build green west green b",
    "active_at": "eta entry test build ent"
   },
   {
    "kind": "ready",
    "id": "theta south red theta alp",
    "rule_id": "value beta check gr",
    "version": 8,
    "priority": 87,
    "name": "omega west red entr",
    "description": "omega record beta alpha beta test status test north alpha east item blue east done zeta omega",
    "source": "gamma ready south lambda item status lambda zeta value kappa north east done ready iota entry red kappa epsilon entry kappa lambda blue kappa check status item alpha west epsilon north north item kapp",
    "source_sha256": "eta status south beta test north ready delta beta lambda value g",
    "created_by": "entry east west iota kapp",
    "created_at": "lambda record iota zeta",
    "active_at": "build test south iota va"
   },
   {
    "kind": "gamma",
    "id": "ready beta check north we",
    "rule_id": "east item status d",
    "version": 1,
    "priority": 96,
    "name": "epsilon lambda ite",
    "description": "entry west test done red south kappa blue eta value north blue item record iota north test gamma north record test value lambda omega status status kappa omega theta entry kappa blue ready entry red l",
    "source": "delta red omega theta lambda west test zeta beta alpha status west eta done item blue status red green entry kappa record alpha red iota done west build test record alpha status iota lambda delta kapp",
    "source_sha256": "value check done value record theta lambda beta status lambda de",
    "created_by": "beta west done alpha lamb",
    "created_at": "status blue ready delta",
    "active_at": "status ready build south"
   },
   {
    "kind": "theta",
    "id": "red lambda beta value the",
    "rule_id": "green record test beta north entry",
    "version": 8,
    "priority": 8202,
    "name": "east north gamma west beta entry d",
    "description": "kappa red ready west theta beta lambda alpha south status north omega value test status item north ready iota south omega build south delta red gamma status d",
    "source": "theta west entry status delta blue south value build entry eta test beta beta done beta build beta ready delta delta check beta entry eta value check red ready north theta check iota green alpha omega",
    "source_sha256": "south east build value red east gamma blue beta red status omega",
    "created_by": "lambda omega beta lambda",
    "created_at": "zeta beta record west re",
    "active_at": "epsilon ready build item"
   }
  ]
 }
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const rr = load("rr"), wr = load("wr");
let out = "";
for (const r of rr.rules) out += `\n=== ROUTING ${r.rule_id} v${r.version}\n${JSON.stringify(r.config_schema)}\n${r.source}\n`;
for (const r of wr.rules.filter(r=>r.kind==="custom")) out += `\n=== REWRITE ${r.rule_id} v${r.version}\n${r.source}\n`;
return out;
})(fake, fakeLoad);
