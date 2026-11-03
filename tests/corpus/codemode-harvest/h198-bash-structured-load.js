// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "status check east item test kappa entry 1009\nblue iota test red\nblue north value east build south alpha 1833\nomega item eta eta ready iota beta 155\nitem done alpha\nready red\nstatus theta status beta delta red\neta test kappa test west done 222\nblue item blue 5036\ngamma item kappa theta gamma 3432\nentry theta check test eta alpha alpha alpha",
    "truncated": false
   }
  }
 ]
};
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
 "reportSessionEntries": [
  {
   "type": "theta o",
   "version": 2,
   "id": "epsilon theta west delta lambda red",
   "timestamp": "value delta east gamma i",
   "cwd": "gamma zeta north blue check i"
  },
  {
   "type": "blue south t",
   "id": "epsilon",
   "parentId": null,
   "timestamp": "north build status red t",
   "provider": "omega",
   "modelId": "test south"
  },
  {
   "type": "entry done delta stat",
   "id": "entry la",
   "parentId": "eta gamm",
   "timestamp": "check alpha iota item ea",
   "thinkingLevel": "gree"
  },
  {
   "type": "eta alp",
   "id": "test del",
   "parentId": "north bu",
   "timestamp": "theta build test value l",
   "message": {
    "role": "item g",
    "content": "",
    "sections": {
     "preamble": "status status lambda red status green zeta gamma north entry delta check blue south theta blue gamma kappa west delta north eta ready value gamma alpha iota iota ready build iota kappa gamma south ite",
     "project_context": "lambda south build check value green south build kappa west lambda build red record kappa green north theta west done omega red build lambda epsilon delta value check blue east eta iota blue zeta thet",
     "skills": "epsilon entry kappa test epsilon lambda lambda omega zeta lambda entry gamma test theta epsilon delta kappa south record south south item delta zeta east alpha red done entry ready gamma test status e",
     "cwd": "east delta record delta record eta red lam"
    },
    "timestamp": 1166715500182,
    "toolsAdded": [
     {
      "name": "lamb",
      "description": "check kappa green zeta eta lambda beta green test done record iota record east east zeta record blue eta done status theta north status green epsilon gamma ready done test check kappa status record al",
      "parameters": {
       "type": "west b",
       "required": [
        "beta"
       ],
       "properties": {
        "path": {
         "type": "iota o",
         "description": "gamma eta status item omega test status theta l"
        },
        "offset": {
         "type": "beta c",
         "description": "done epsilon eta green north item north entry"
        },
        "limit": {
         "type": "west g",
         "description": "kappa check done blue check blu"
        }
       }
      },
      "constrainedSampling": {
       "type": "eta value i",
       "strict": "build"
      }
     },
     {
      "name": "west",
      "description": "zeta done kappa alpha record build blue south status kappa record red theta theta theta done eta alpha entry alpha west omega theta green gamma green south check eta south check done red iota record b",
      "parameters": {
       "type": "beta b",
       "required": [
        "record"
       ],
       "properties": {
        "command": {
         "type": "entry",
         "description": "north test item status e"
        },
        "timeout": {
         "type": "build",
         "description": "north eta ready blue red alpha north omega build"
        }
       }
      },
      "constrainedSampling": {
       "type": "lambda omeg",
       "strict": "epsilo"
      }
     },
     {
      "name": "alpha bl",
      "description": "west south east build blue entry record check west blue build beta test item red omega alpha lambda west kappa theta lambda check entry iota delta zeta alpha test status test beta green west item item",
      "parameters": {
       "type": "test k",
       "required": [
        "red"
       ],
       "properties": {
        "code": {
         "type": "delta",
         "description": "green iota epsilon rec"
        }
       }
      },
      "constrainedSampling": {
       "type": "north e",
       "variants": {
        "gugofi_lark": "green epsilon done west green beta kappa red omega blue omega blue beta green zeta alpha value alpha green blue eta epsilon ready green red south kappa record lambda west eta entry bet"
       }
      }
     }
    ]
   }
  },
  {
   "type": "done ch",
   "id": "zeta rea",
   "parentId": "entry et",
   "timestamp": "entry done lambda blue r",
   "message": {
    "role": "eta",
    "content": [
     {
      "type": "iota",
      "text": "ready alpha east south alpha gamma build entry value south lambda a"
     }
    ],
    "timestamp": 7014073576180
   }
  },
  {
   "type": "iota om",
   "id": "theta so",
   "parentId": "red stat",
   "timestamp": "lambda green green east",
   "message": {
    "role": "value don",
    "content": [
     {
      "type": "beta ite",
      "thinking": "",
      "thinkingSignature": "blue entry ready status south ready iota south eta green iota value check theta epsilon ready build lambda eta west north entry gamma entry done lambda west red east build blue test iota west record i"
     },
     {
      "type": "delt",
      "text": "value north delta north zeta done omega check north north theta red chec",
      "textSignature": "entry alpha delta epsilon red blue item zeta eta eta red delta status eta eta green theta"
     },
     {
      "type": "west sta",
      "id": "east zeta epsilon omega entry red iota entry check red west west entry item red east",
      "name": "item don",
      "arguments": {
       "code": "omega entry west ready record east alpha build gamma check check eta epsilon iota lambda check status iota iota red east lambda theta value south alpha beta entry value east done zeta east epsilon the"
      }
     }
    ],
    "api": "gamma zeta lambd",
    "provider": "south",
    "model": "item omega",
    "usage": {
     "input": 8586,
     "output": 919,
     "cacheRead": 5216,
     "cacheWrite": 4,
     "reasoning": 40,
     "totalTokens": 6259,
     "cost": {
      "input": 0.355,
      "output": 0.567,
      "cacheRead": 0.464,
      "cacheWrite": 2,
      "total": 0.675
     }
    },
    "stopReason": "entry e",
    "timestamp": 7085349840255,
    "responseId": "omega green done check gamma north beta gamma red statu",
    "rawStopReason": "delta eas",
    "durationMs": 4430,
    "thinkingLevel": "red"
   }
  },
  {
   "type": "test ga",
   "id": "blue sou",
   "parentId": "red iota",
   "timestamp": "gamma alpha check kappa",
   "message": {
    "role": "record val",
    "toolCallId": "status status epsilon west south omega west entry north alpha item check north entry",
    "toolName": "omega en",
    "content": [
     {
      "type": "entr",
      "text": "south build iota build blue beta omega record b"
     },
     {
      "type": "alph",
      "text": "west green entry kappa red epsilon delta ready value item south entry done status gamma zeta check record omega value red theta iota entry kappa south omega alpha blue zeta west lambda zeta green gamm"
     }
    ],
    "details": {
     "calls": [
      {
       "id": "ready item alpha red theta east value delta entry west build value red kappa north lam",
       "name": "omeg",
       "args": "status theta west done ready eta omega omega eta blue blue s",
       "status": "ze",
       "durationMs": 4.53
      },
      {
       "id": "done value value north kappa eta status ready theta west zeta blue test eta green sout",
       "name": "eta",
       "args": "theta done west record alpha west beta theta omega blue eta value blue iota green north item alpha zeta build red omega",
       "status": "al",
       "durationMs": 38.734
      }
     ]
    },
    "isError": false,
    "durationMs": 71,
    "timestamp": 8547534248660,
    "nestedCalls": {
     "calls": [
      {
       "id": "zeta item iota zeta west alpha status epsilon iota value status blue north beta status",
       "name": "test",
       "status": "ea",
       "arguments": {
        "path": "blue record delta north status iota kappa status"
       },
       "durationMs": 1
      },
      {
       "id": "west status delta green value item entry test delta record east value east delta beta",
       "name": "zeta",
       "status": "ka",
       "arguments": {
        "command": "epsilon theta item done lambda record ready omega south blue epsilon test red kappa status ga",
        "timeout": 76
       },
       "durationMs": 45
      }
     ],
     "complete": true
    }
   }
  },
  {
   "type": "beta re",
   "id": "zeta rec",
   "parentId": "east eas",
   "timestamp": "alpha zeta epsilon red r",
   "message": {
    "role": "kappa tes",
    "content": [
     {
      "type": "kappa bl",
      "thinking": "",
      "thinkingSignature": "gamma eta north omega test lambda beta zeta alpha ready entry gamma omega green done east done theta north status done item item omega east item blue entry epsilon item epsilon gamma iota build eta la"
     },
     {
      "type": "beta del",
      "id": "kappa gamma record entry kappa delta blue eta item entry east beta green gamma red b",
      "name": "east iot",
      "arguments": {
       "code": "west theta entry ready test theta east delta ready check green theta test eta entry west west iota beta gamma kappa check south theta delta gamma kappa south lambda west east red lambda zeta east epsi"
      }
     }
    ],
    "api": "value item zeta",
    "provider": "green",
    "model": "theta build",
    "usage": {
     "input": 5002,
     "output": 428,
     "cacheRead": 8324,
     "cacheWrite": 2,
     "reasoning": 82,
     "totalTokens": 97565,
     "cost": {
      "input": 0.772,
      "output": 0.128,
      "cacheRead": 0.748,
      "cacheWrite": 6,
      "total": 0.598
     }
    },
    "stopReason": "west re",
    "timestamp": 5042154991144,
    "responseId": "zeta red red alpha value red eta status check status so",
    "rawStopReason": "east delt",
    "durationMs": 7994,
    "thinkingLevel": "nort"
   }
  },
  {
   "type": "build a",
   "id": "check io",
   "parentId": "beta ome",
   "timestamp": "delta lambda red ready t",
   "message": {
    "role": "theta beta",
    "toolCallId": "done south theta alpha value eta east build record eta status record check build bui",
    "toolName": "south no",
    "content": [
     {
      "type": "test",
      "text": "done entry iota alpha gamma omega gamma beta th"
     },
     {
      "type": "chec",
      "text": "epsilon north item ready test epsilon test record delta done gamma delta test east alpha zeta value delta record ready build entry value iota theta ready omega omega blue epsilon iota omega done beta"
     }
    ],
    "details": {
     "calls": [
      {
       "id": "check build north entry build epsilon iota item south gamma build entry test eta alpha",
       "name": "lamb",
       "args": "theta eta iota status omega eta iota west south d",
       "status": "bl",
       "durationMs": 2.742
      },
      {
       "id": "red east eta west status status blue test south ready gamma west omega lambda green re",
       "name": "delt",
       "args": "build test value status record south west lambda theta north the",
       "status": "ep",
       "durationMs": 1.8
      },
      {
       "id": "south east theta done test entry kappa entry done check item zeta north theta check so",
       "name": "reco",
       "args": "iota beta south green blue epsilon blue status iota eta entry delta north omega omega lambda omega eta zeta done test theta ready south done east north west south west east omega status green eta omeg",
       "status": "ch",
       "durationMs": 47.407
      }
     ],
     "fullOutputPath": "gamma kappa kappa status zeta blue om"
    },
    "isError": false,
    "durationMs": 94,
    "timestamp": 7712714652501,
    "nestedCalls": {
     "calls": [
      {
       "id": "delta check green entry check check build theta east test ready zeta done iota test be",
       "name": "item",
       "status": "re",
       "arguments": {
        "path": "build red east build west red west eta"
       },
       "durationMs": 4
      },
      {
       "id": "epsilon beta beta status zeta entry blue kappa north beta eta red south epsilon delta",
       "name": "chec",
       "status": "bl",
       "arguments": {
        "path": "delta west item eta delta epsilon value beta blue sta"
       },
       "durationMs": 5
      },
      {
       "id": "epsilon epsilon red epsilon theta ready lambda lambda ready south status status north",
       "name": "read",
       "status": "bu",
       "arguments": {
        "command": "check epsilon south red kappa lambda omega eta alpha kappa eta west beta item north kappa ready west south alpha kappa delta ready beta delta beta alpha red test done value delta eta record record eas",
        "timeout": 63
       },
       "durationMs": 23
      }
     ],
     "complete": true
    }
   }
  },
  {
   "type": "west ga",
   "id": "west don",
   "parentId": "north re",
   "timestamp": "west eta gamma record et",
   "message": {
    "role": "test epsi",
    "content": [
     {
      "type": "alpha va",
      "thinking": "",
      "thinkingSignature": "west record blue theta item gamma build gamma theta epsilon lambda blue kappa item zeta status beta delta done iota done blue blue epsilon west red east test build gamma east ready lambda alpha item l"
     },
     {
      "type": "theta th",
      "id": "record zeta record gamma eta check entry check theta value check north build record",
      "name": "green ep",
      "arguments": {
       "code": "blue blue zeta beta red build test red record epsilon gamma record item north blue south green beta alpha value south theta eta south beta green entry item eta item ready omega zeta green test epsilon"
      }
     }
    ],
    "api": "entry iota recor",
    "provider": "value",
    "model": "eta theta b",
    "usage": {
     "input": 89358,
     "output": 546,
     "cacheRead": 6273,
     "cacheWrite": 9,
     "reasoning": 23,
     "totalTokens": 25114,
     "cost": {
      "input": 0.144,
      "output": 0.59,
      "cacheRead": 0.856,
      "cacheWrite": 7,
      "total": 1.0
     }
    },
    "stopReason": "iota io",
    "timestamp": 3058919587843,
    "responseId": "entry ready lambda zeta build west delta iota west test",
    "rawStopReason": "alpha che",
    "durationMs": 49876,
    "thinkingLevel": "alph"
   }
  },
  {
   "type": "test do",
   "id": "theta re",
   "parentId": "eta beta",
   "timestamp": "kappa value green theta",
   "message": {
    "role": "build sout",
    "toolCallId": "check omega build iota kappa ready item south done north epsilon build entry epsilon",
    "toolName": "theta bl",
    "content": [
     {
      "type": "beta",
      "text": "eta red lambda test delta check value item nort"
     },
     {
      "type": "alph",
      "text": "check ready kappa eta blue lambda item entry delta eta delta blue item delta epsilon status test record north kappa green beta status value south done gamma red alpha alpha eta delta east test item io"
     }
    ],
    "details": {
     "calls": [
      {
       "id": "lambda green entry build green ready item ready epsilon theta lambda east lambda delta",
       "name": "east",
       "args": "gamma ready omega test record build done check",
       "status": "be",
       "durationMs": 8.646
      },
      {
       "id": "alpha gamma alpha blue lambda east item item item beta iota red eta blue gamma entry l",
       "name": "valu",
       "args": "gamma gamma done zeta red green omega west check check alpha kappa",
       "status": "we",
       "durationMs": 4.175
      },
      {
       "id": "eta build red eta red east check green iota blue theta east lambda ready test blue bet",
       "name": "kapp",
       "args": "green entry test red build gamma ready va",
       "status": "be",
       "durationMs": 0.609
      }
     ],
     "fullOutputPath": "iota record theta alpha record west d"
    },
    "isError": false,
    "durationMs": 37,
    "timestamp": 2599166906017,
    "nestedCalls": {
     "calls": [
      {
       "id": "west iota theta south value item value record kappa item gamma east lambda gamma beta",
       "name": "epsi",
       "status": "gr",
       "arguments": {
        "path": "gamma delta value done value blue s"
       },
       "durationMs": 9
      },
      {
       "id": "lambda kappa iota blue item test zeta red status gamma blue theta green iota alpha gre",
       "name": "blue",
       "status": "re",
       "arguments": {
        "path": "status done theta epsilon value delta done delta kappa l"
       },
       "durationMs": 7
      },
      {
       "id": "green north red gamma epsilon west alpha green north zeta theta build epsilon kappa no",
       "name": "entr",
       "status": "de",
       "arguments": {
        "path": "record kappa east theta build"
       },
       "durationMs": 8
      }
     ],
     "complete": true
    }
   }
  },
  {
   "type": "epsilon",
   "id": "lambda b",
   "parentId": "test wes",
   "timestamp": "east alpha north build t",
   "message": {
    "role": "item test",
    "content": [
     {
      "type": "record d",
      "thinking": "",
      "thinkingSignature": "alpha entry epsilon entry item zeta blue north build alpha north iota theta item value entry lambda zeta item gamma value value kappa omega gamma check blue epsilon lambda build beta item record epsil"
     },
     {
      "type": "epsilon",
      "id": "theta kappa north record gamma beta red beta alpha theta alpha done item green epsil",
      "name": "theta ka",
      "arguments": {
       "code": "entry omega ready gamma build kappa value check entry east epsilon red record red value omega value check status red record check entry check west north north done item value epsilon red delta test gr"
      }
     }
    ],
    "api": "north epsilon it",
    "provider": "north",
    "model": "blue status",
    "usage": {
     "input": 60759,
     "output": 145,
     "cacheRead": 95143,
     "cacheWrite": 8,
     "reasoning": 65,
     "totalTokens": 46454,
     "cost": {
      "input": 0.436,
      "output": 0.357,
      "cacheRead": 0.294,
      "cacheWrite": 4,
      "total": 0.169
     }
    },
    "stopReason": "zeta en",
    "timestamp": 6515455314661,
    "responseId": "west blue eta record item omega item north west record",
    "rawStopReason": "south bet",
    "durationMs": 62928,
    "thinkingLevel": "blue"
   }
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
text((await tools.bash({command:"wc -l zusazoni/qan1/fov/qan2/qan3/lofu4.ts zusazoni/qan3/REVITI.md zusazoni/qan1/munu/{qar,subekebo}.md; rg -n 'QASA.rirulopor|devemo.busuf|mogobu|losobudaf|dagasoDoro|doro\\(busuf\\)|rirulopor' zusazoni/qan1/fov/qan2/qan3/lofu4.ts zusazoni/qan3/bulu/lofu5.ts zusazoni/qan3/fov/qan6/lofu7.ts",timeout:60})).output);
const entries=load("reportSessionEntries");
for (const e of entries.filter(e=>e.message?.role==="toolResult")) {
 const texts=e.message.content.filter(c=>c.type==="text").map(c=>c.text).join("\n");
 const i=texts.indexOf('{"output":');
 text({id:e.id,chars:texts.length,actualTabs:(texts.match(/\t/g)||[]).length,literalTabEscapes:(texts.match(/\\t/g)||[]).length,literalVopadubEscapes:(texts.match(/\\n/g)||[]).length,escapedOutputStart:i,example:i>=0?texts.slice(i,i+500):undefined,detailsKeys:Object.keys(e.message.details??{})});
}
text(load("reportSessionEntries").find(e=>e.message?.role==="toolResult").message.details);
})(fake, fakeLoad);
