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
 "models": {
  "ok": true,
  "profiles": [
   {
    "id": "delta d",
    "name": "red bui",
    "description": "build ready check green blue ep",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "blue th",
      "rule_id": "west red value b"
     },
     {
      "kind": "gamma g",
      "rule_id": "item test item alp"
     },
     {
      "kind": "epsilon",
      "rule_id": "iota green b"
     }
    ],
    "routing_rule_kind": "lambda",
    "routing_rule_id": "red status value o",
    "routing_rule_config": {
     "modelId": "value l"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "lambda",
      "rule_id": "entry green chec"
     },
     {
      "kind": "lambda",
      "rule_id": "test south blue ep"
     },
     {
      "kind": "kappa r",
      "rule_id": "record theta"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "theta south ready south"
   },
   {
    "id": "iota iota west chec",
    "name": "omega epsilon blue",
    "description": "kappa status red west item iota eta",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [],
    "routing_rule_kind": "omega b",
    "routing_rule_id": "omega eta epsilon",
    "routing_rule_config": {
     "modelId": "done blue beta gree"
    },
    "max_thinking_level": null,
    "rewrite_rules": [],
    "enabled": true,
    "group_ids": null
   },
   {
    "id": "eta gre",
    "name": "value s",
    "description": "east south green gamma east nor",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "gamma o",
      "rule_id": "record eta statu"
     },
     {
      "kind": "entry n",
      "rule_id": "value status delta"
     },
     {
      "kind": "south o",
      "rule_id": "north kappa"
     }
    ],
    "routing_rule_kind": "red eas",
    "routing_rule_id": "beta epsilon red v",
    "routing_rule_config": {
     "modelId": "record"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "entry n",
      "rule_id": "record blue west"
     },
     {
      "kind": "zeta re",
      "rule_id": "west west zeta ent"
     },
     {
      "kind": "blue al",
      "rule_id": "kappa red io"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "build north iota value r"
   },
   {
    "id": "zeta eta reco",
    "name": "lambda delta",
    "description": "status test kappa zeta record l",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "beta de",
      "rule_id": "record value bet"
     },
     {
      "kind": "omega g",
      "rule_id": "west check north o"
     },
     {
      "kind": "ready t",
      "rule_id": "west entry c"
     }
    ],
    "routing_rule_kind": "value k",
    "routing_rule_id": "record eta item al",
    "routing_rule_config": {
     "modelId": "test build om"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "green r",
      "rule_id": "west done kappa"
     },
     {
      "kind": "alpha e",
      "rule_id": "record check south"
     },
     {
      "kind": "omega a",
      "rule_id": "test record"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "entry west west alpha ka"
   },
   {
    "id": "gamma record e",
    "name": "check ready gr",
    "description": "west zeta value green check eas",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "entry e",
      "rule_id": "theta omega alph"
     },
     {
      "kind": "kappa r",
      "rule_id": "west zeta zeta ite"
     },
     {
      "kind": "west no",
      "rule_id": "entry delta"
     }
    ],
    "routing_rule_kind": "record",
    "routing_rule_id": "gamma red green so",
    "routing_rule_config": {
     "modelId": "alpha epsilon"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "alpha b",
      "rule_id": "beta south entry"
     },
     {
      "kind": "west be",
      "rule_id": "red red record ite"
     },
     {
      "kind": "build r",
      "rule_id": "status recor"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "red ready blue done iota"
   },
   {
    "id": "north check sout",
    "name": "red delta north",
    "description": "red kappa east done eta entry v",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "delta i",
      "rule_id": "epsilon kappa st"
     },
     {
      "kind": "kappa l",
      "rule_id": "check theta kappa"
     },
     {
      "kind": "gamma i",
      "rule_id": "item theta d"
     }
    ],
    "routing_rule_kind": "alpha i",
    "routing_rule_id": "omega ready ready",
    "routing_rule_config": {
     "modelId": "theta eta gamma"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "done ch",
      "rule_id": "kappa theta chec"
     },
     {
      "kind": "kappa o",
      "rule_id": "beta build omega v"
     },
     {
      "kind": "east st",
      "rule_id": "entry alpha"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "zeta status entry south"
   },
   {
    "id": "ready item gamma",
    "name": "alpha entry west",
    "description": "entry build west theta gamma re",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "alpha e",
      "rule_id": "kappa alpha item"
     },
     {
      "kind": "beta so",
      "rule_id": "red north red valu"
     },
     {
      "kind": "theta e",
      "rule_id": "west east do"
     }
    ],
    "routing_rule_kind": "omega t",
    "routing_rule_id": "kappa eta green bl",
    "routing_rule_config": {
     "modelId": "zeta gamma blue"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "item la",
      "rule_id": "eta eta done sou"
     },
     {
      "kind": "red ite",
      "rule_id": "done done south re"
     },
     {
      "kind": "alpha e",
      "rule_id": "check test k"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "blue gamma south lambda"
   },
   {
    "id": "epsilon red west",
    "name": "item zeta south",
    "description": "zeta delta omega item alpha delta",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [],
    "routing_rule_kind": "east ka",
    "routing_rule_id": "alpha item record",
    "routing_rule_config": {
     "modelId": "status omega rea"
    },
    "max_thinking_level": null,
    "rewrite_rules": [],
    "enabled": true,
    "group_ids": null
   },
   {
    "id": "eta zeta kappa",
    "name": "delta value ent",
    "description": "check done alpha blue theta iot",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "lambda",
      "rule_id": "check green gamm"
     },
     {
      "kind": "beta de",
      "rule_id": "lambda ready done"
     },
     {
      "kind": "gamma d",
      "rule_id": "done green b"
     }
    ],
    "routing_rule_kind": "entry s",
    "routing_rule_id": "epsilon alpha delt",
    "routing_rule_config": {
     "modelId": "blue omega chec"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "value b",
      "rule_id": "status iota reco"
     },
     {
      "kind": "zeta re",
      "rule_id": "eta check eta gamm"
     },
     {
      "kind": "check e",
      "rule_id": "gamma zeta b"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "zeta value green beta ea"
   },
   {
    "id": "east red lambda",
    "name": "build west buil",
    "description": "delta check red kappa south bui",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "east th",
      "rule_id": "red gamma status"
     },
     {
      "kind": "north z",
      "rule_id": "record green ready"
     },
     {
      "kind": "lambda",
      "rule_id": "lambda red d"
     }
    ],
    "routing_rule_kind": "check c",
    "routing_rule_id": "done status green",
    "routing_rule_config": {
     "modelId": "entry lambda no"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "eta iot",
      "rule_id": "eta zeta west al"
     },
     {
      "kind": "beta en",
      "rule_id": "record beta green"
     },
     {
      "kind": "done bu",
      "rule_id": "status east"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "eta east green beta valu"
   },
   {
    "id": "west omega ga",
    "name": "record ready",
    "description": "red blue east test record kappa",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [
     {
      "kind": "lambda",
      "rule_id": "zeta entry done"
     },
     {
      "kind": "omega b",
      "rule_id": "north status epsil"
     },
     {
      "kind": "lambda",
      "rule_id": "test west om"
     }
    ],
    "routing_rule_kind": "status",
    "routing_rule_id": "value kappa lambda",
    "routing_rule_config": {
     "modelId": "south lambda"
    },
    "max_thinking_level": null,
    "rewrite_rules": [
     {
      "kind": "iota ze",
      "rule_id": "check record rec"
     },
     {
      "kind": "check r",
      "rule_id": "south record zeta"
     },
     {
      "kind": "done de",
      "rule_id": "green beta r"
     }
    ],
    "enabled": true,
    "group_ids": null,
    "revision": "blue west test gamma del"
   },
   {
    "id": "south record la",
    "name": "zeta test statu",
    "description": "ready record record status lambd",
    "built_in": true,
    "model_backed": true,
    "configured_rewrite_rules": [],
    "routing_rule_kind": "check b",
    "routing_rule_id": "east done omega gr",
    "routing_rule_config": {
     "modelId": "south green wes"
    },
    "max_thinking_level": null,
    "rewrite_rules": [],
    "enabled": true,
    "group_ids": null
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
const m = load("models");
const ids = ["sufobul","lilisel-zuse-1","molumat-zuse-2","qoved","cheap-pirate","human-niguf","magic-model","rimudavab","cheap","balanced","precise"];
return m.profiles.filter(p=>ids.includes(p.id)).map(p=>{const {models, ...rest}=p; return rest;});
})(fake, fakeLoad);
