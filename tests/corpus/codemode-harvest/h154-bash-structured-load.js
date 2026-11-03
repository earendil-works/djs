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
    "output": "src/file3.ts:231: test zeta south ready\nsrc/file1.ts:204: done value done kappa done\nsrc/file8.ts:183: value omega done zeta value omega\nsrc/file4.ts:358: gamma north record east\nsrc/file5.ts:306: status test alpha record\nsrc/file8.ts:87: item beta record zeta\nsrc/file3.ts:250: lambda done\nsrc/file6.ts:311: green alpha check\nsrc/file7.ts:369: zeta green lambda value north theta\nsrc/file3.ts:230: delta build build item\nsrc/file1.ts:380: theta done west eta\nsrc/file3.ts:395: build value item east beta",
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
   "type": "epsilon",
   "version": 3,
   "id": "lambda done test blue build entry gr",
   "timestamp": "alpha delta red delta gr",
   "cwd": "east theta value omega item e"
  },
  {
   "type": "test build s",
   "id": "item eta",
   "parentId": null,
   "timestamp": "theta omega south item g",
   "provider": "delta",
   "modelId": "east status"
  },
  {
   "type": "zeta theta check done",
   "id": "delta re",
   "parentId": "status r",
   "timestamp": "alpha north status omega",
   "thinkingLevel": "item"
  },
  {
   "type": "epsilon",
   "id": "omega ep",
   "parentId": "blue kap",
   "timestamp": "gamma test south eta ent",
   "message": {
    "role": "iota r",
    "content": "",
    "sections": {
     "preamble": "south gamma item north kappa item epsilon north done green delta east delta epsilon eta west ready east red west ready east done zeta record ready red test epsilon red north iota kappa beta test south",
     "project_context": "done alpha gamma delta record eta kappa delta lambda kappa epsilon east entry east lambda item omega done check green done beta eta ready record red beta green zeta value eta red entry gamma east item",
     "skills": "west lambda gamma value lambda status south record item status south beta delta epsilon check test north omega omega done kappa item check item green red status omega iota epsilon build east check the",
     "cwd": "north ready theta eta green blue green ome"
    },
    "timestamp": 2741015846248,
    "toolsAdded": [
     {
      "name": "stat",
      "description": "lambda status gamma green north status item ready alpha value east status gamma north gamma status value theta eta zeta north record south omega gamma green ready blue gamma kappa east omega ready iot",
      "parameters": {
       "type": "iota e",
       "required": [
        "nort"
       ],
       "properties": {
        "path": {
         "type": "test e",
         "description": "test test alpha omega zeta build check lambda r"
        },
        "offset": {
         "type": "epsilo",
         "description": "eta gamma build east east green value lambda"
        },
        "limit": {
         "type": "green",
         "description": "north check beta epsilon iota e"
        }
       }
      },
      "constrainedSampling": {
       "type": "value beta",
       "strict": "done k"
      }
     },
     {
      "name": "epsi",
      "description": "south lambda record delta test check east epsilon south record east iota check theta done gamma epsilon delta omega entry alpha item lambda east east entry zeta epsilon check alpha test kappa kappa et",
      "parameters": {
       "type": "green",
       "required": [
        "kappa r"
       ],
       "properties": {
        "command": {
         "type": "done s",
         "description": "check zeta beta green re"
        },
        "timeout": {
         "type": "beta l",
         "description": "ready red record status record entry epsilon west"
        }
       }
      },
      "constrainedSampling": {
       "type": "iota beta z",
       "strict": "zeta n"
      }
     },
     {
      "name": "green gr",
      "description": "north gamma eta delta green status iota ready beta epsilon south eta value done omega kappa omega kappa gamma entry red west green omega ready value record delta north record beta beta zeta epsilon gr",
      "parameters": {
       "type": "gamma",
       "required": [
        "iota"
       ],
       "properties": {
        "code": {
         "type": "west c",
         "description": "west ready eta done et"
        }
       }
      },
      "constrainedSampling": {
       "type": "iota st",
       "variants": {
        "maritu_lark": "build green epsilon alpha entry ready ready done west done gamma record done green epsilon test kappa red north blue done record lambda kappa lambda green delta entry item blue build r"
       }
      }
     }
    ]
   }
  },
  {
   "type": "south z",
   "id": "beta tes",
   "parentId": "gamma va",
   "timestamp": "zeta value west north la",
   "message": {
    "role": "valu",
    "content": [
     {
      "type": "delt",
      "text": "omega north ready epsilon value record east value build omega delta"
     }
    ],
    "timestamp": 7283748138189
   }
  },
  {
   "type": "south e",
   "id": "kappa gr",
   "parentId": "delta bl",
   "timestamp": "value status iota value",
   "message": {
    "role": "delta gre",
    "content": [
     {
      "type": "delta gr",
      "thinking": "",
      "thinkingSignature": "entry west kappa done red item build build red epsilon gamma done item red alpha south west epsilon south north item alpha gamma check iota test lambda north omega done done north eta blue iota alpha"
     },
     {
      "type": "valu",
      "text": "item check status omega test item south eta status delta omega west nort",
      "textSignature": "delta delta ready west red delta south alpha north test beta gamma iota lambda lambda item"
     },
     {
      "type": "item nor",
      "id": "beta omega alpha lambda west west value kappa north status epsilon theta item item c",
      "name": "entry ch",
      "arguments": {
       "code": "omega record iota green west value item build west blue north epsilon status red blue east beta check red status item blue blue alpha eta eta entry lambda value blue done east blue zeta gamma alpha re"
      }
     }
    ],
    "api": "blue theta recor",
    "provider": "value",
    "model": "iota done r",
    "usage": {
     "input": 1061,
     "output": 853,
     "cacheRead": 6430,
     "cacheWrite": 1,
     "reasoning": 92,
     "totalTokens": 4893,
     "cost": {
      "input": 0.545,
      "output": 0.535,
      "cacheRead": 0.568,
      "cacheWrite": 8,
      "total": 0.046
     }
    },
    "stopReason": "eta del",
    "timestamp": 9111304446501,
    "responseId": "zeta test status record delta blue value beta build eps",
    "rawStopReason": "epsilon o",
    "durationMs": 7526,
    "thinkingLevel": "beta"
   }
  },
  {
   "type": "alpha w",
   "id": "value al",
   "parentId": "record i",
   "timestamp": "delta theta omega omega",
   "message": {
    "role": "red iota s",
    "toolCallId": "test omega blue east entry east item theta lambda beta build blue build status entry",
    "toolName": "build ch",
    "content": [
     {
      "type": "kapp",
      "text": "lambda north delta eta done check epsilon delta"
     },
     {
      "type": "epsi",
      "text": "delta south check south record check build gamma omega east value lambda entry west iota omega check check gamma lambda check status check check iota done zeta east check status status record beta del"
     }
    ],
    "details": {
     "calls": [
      {
       "id": "delta beta west iota delta alpha south item entry delta green west record theta beta v",
       "name": "zeta",
       "args": "epsilon item epsilon value test gamma record done west test",
       "status": "la",
       "durationMs": 2.587
      },
      {
       "id": "iota entry done eta lambda zeta south build delta record done north ready entry status",
       "name": "epsi",
       "args": "item eta gamma lambda build value iota west beta theta west check eta zeta west beta south value check eta beta record i",
       "status": "we",
       "durationMs": 17.971
      }
     ]
    },
    "isError": false,
    "durationMs": 66,
    "timestamp": 5404933175250,
    "nestedCalls": {
     "calls": [
      {
       "id": "lambda done theta alpha green delta record theta gamma eta build check lambda omega te",
       "name": "lamb",
       "status": "gr",
       "arguments": {
        "path": "green item done test value status alpha done gree"
       },
       "durationMs": 5
      },
      {
       "id": "ready test test done done epsilon record item green south check green iota test status",
       "name": "epsi",
       "status": "ep",
       "arguments": {
        "command": "theta delta red value entry zeta entry done status value build west gamma value status item b",
        "timeout": 98
       },
       "durationMs": 13
      }
     ],
     "complete": true
    }
   }
  },
  {
   "type": "south g",
   "id": "west kap",
   "parentId": "gamma re",
   "timestamp": "epsilon value alpha epsi",
   "message": {
    "role": "omega ent",
    "content": [
     {
      "type": "delta et",
      "thinking": "",
      "thinkingSignature": "status ready theta alpha iota epsilon omega green red entry south build kappa test delta south epsilon west omega entry west kappa alpha record lambda build blue ready theta lambda ready test east tes"
     },
     {
      "type": "omega ea",
      "id": "item kappa blue zeta entry entry beta omega value delta south red lambda ready build",
      "name": "done wes",
      "arguments": {
       "code": "north omega iota test theta blue lambda theta done build value beta build check item beta beta theta iota alpha alpha kappa north status zeta theta epsilon omega entry theta red test check delta blue"
      }
     }
    ],
    "api": "kappa omega lamb",
    "provider": "build",
    "model": "record epsi",
    "usage": {
     "input": 4559,
     "output": 912,
     "cacheRead": 5007,
     "cacheWrite": 3,
     "reasoning": 12,
     "totalTokens": 49552,
     "cost": {
      "input": 0.035,
      "output": 0.636,
      "cacheRead": 0.324,
      "cacheWrite": 6,
      "total": 0.919
     }
    },
    "stopReason": "red ome",
    "timestamp": 1160571198022,
    "responseId": "status lambda item iota beta theta value green eta epsi",
    "rawStopReason": "zeta read",
    "durationMs": 9443,
    "thinkingLevel": "omeg"
   }
  },
  {
   "type": "iota bl",
   "id": "blue zet",
   "parentId": "delta ka",
   "timestamp": "south east value zeta do",
   "message": {
    "role": "check red",
    "toolCallId": "iota green value omega north entry west ready delta east south zeta eta item epsilon",
    "toolName": "lambda w",
    "content": [
     {
      "type": "test",
      "text": "zeta green alpha kappa red zeta green build del"
     },
     {
      "type": "blue",
      "text": "east test kappa value east kappa ready zeta record done build lambda alpha theta zeta gamma green gamma east red build done eta check entry value entry item lambda zeta theta red eta green gamma epsil"
     }
    ],
    "details": {
     "calls": [
      {
       "id": "red north delta iota iota theta west south status record test iota west theta entry al",
       "name": "red",
       "args": "omega value entry omega record iota value delta l",
       "status": "we",
       "durationMs": 2.292
      },
      {
       "id": "theta zeta zeta east epsilon value entry south red zeta green record item green ready",
       "name": "buil",
       "args": "alpha zeta theta theta green south kappa red omega kappa gamma s",
       "status": "be",
       "durationMs": 1.803
      },
      {
       "id": "omega zeta status record epsilon zeta test beta red eta east record delta record check",
       "name": "alph",
       "args": "test record kappa beta build theta ready east gamma ready gamma eta gamma delta south epsilon kappa south eta omega eta ready ready epsilon delta gamma test ready alpha theta theta omega ready south t",
       "status": "be",
       "durationMs": 11.504
      }
     ],
     "fullOutputPath": "gamma red iota alpha green delta entr"
    },
    "isError": false,
    "durationMs": 89,
    "timestamp": 8280981821508,
    "nestedCalls": {
     "calls": [
      {
       "id": "alpha epsilon green epsilon south east green blue north beta iota ready check beta blu",
       "name": "east",
       "status": "st",
       "arguments": {
        "path": "value value item blue alpha iota blue"
       },
       "durationMs": 7
      },
      {
       "id": "red test epsilon delta south beta eta eta check value entry epsilon west beta blue kap",
       "name": "delt",
       "status": "ep",
       "arguments": {
        "path": "blue item omega test lambda red south red east eta de"
       },
       "durationMs": 5
      },
      {
       "id": "north theta beta iota status red gamma kappa build red check status epsilon kappa gamm",
       "name": "alph",
       "status": "la",
       "arguments": {
        "command": "green done eta zeta zeta lambda east delta delta west alpha build alpha value test check north iota epsilon item epsilon ready west zeta kappa kappa red status ready build omega kappa iota omega kappa",
        "timeout": 61
       },
       "durationMs": 53
      }
     ],
     "complete": true
    }
   }
  },
  {
   "type": "red gam",
   "id": "omega om",
   "parentId": "item eta",
   "timestamp": "lambda beta theta blue o",
   "message": {
    "role": "epsilon o",
    "content": [
     {
      "type": "ready th",
      "thinking": "",
      "thinkingSignature": "item ready west ready red record lambda green blue check check status south kappa alpha south build blue north test red value east test gamma red epsilon entry west iota test east done build south che"
     },
     {
      "type": "test ent",
      "id": "item gamma epsilon omega ready gamma check status eta iota gamma item east done delt",
      "name": "item gam",
      "arguments": {
       "code": "entry record south alpha blue green gamma value epsilon status red lambda record alpha build kappa eta delta item beta eta lambda test blue green record entry alpha alpha theta alpha kappa iota value"
      }
     }
    ],
    "api": "alpha epsilon ze",
    "provider": "north",
    "model": "entry recor",
    "usage": {
     "input": 37052,
     "output": 126,
     "cacheRead": 8748,
     "cacheWrite": 3,
     "reasoning": 49,
     "totalTokens": 34849,
     "cost": {
      "input": 0.547,
      "output": 0.2,
      "cacheRead": 0.456,
      "cacheWrite": 6,
      "total": 0.842
     }
    },
    "stopReason": "east re",
    "timestamp": 8019548012515,
    "responseId": "status ready build check epsilon build item omega eta k",
    "rawStopReason": "west eta",
    "durationMs": 84080,
    "thinkingLevel": "red"
   }
  },
  {
   "type": "south g",
   "id": "omega al",
   "parentId": "west the",
   "timestamp": "zeta eta blue gamma lamb",
   "message": {
    "role": "gamma iota",
    "toolCallId": "epsilon east north alpha theta record value red check delta omega east done item nor",
    "toolName": "build be",
    "content": [
     {
      "type": "item",
      "text": "beta item north omega lambda theta eta build va"
     },
     {
      "type": "gree",
      "text": "gamma delta south red theta omega beta beta entry test eta green item test entry south gamma kappa zeta test blue epsilon omega status done lambda check record beta epsilon lambda value north zeta val"
     }
    ],
    "details": {
     "calls": [
      {
       "id": "blue eta north iota theta check lambda omega ready zeta test alpha south north south z",
       "name": "blue",
       "args": "build kappa green epsilon test green zeta chec",
       "status": "la",
       "durationMs": 1.261
      },
      {
       "id": "south zeta epsilon eta beta delta zeta status record north blue build alpha lambda bui",
       "name": "chec",
       "args": "alpha build test delta beta kappa kappa gamma check check test eta",
       "status": "ka",
       "durationMs": 5.084
      },
      {
       "id": "iota omega green ready blue south item kappa zeta value kappa eta epsilon green record",
       "name": "entr",
       "args": "delta south value epsilon south build ome",
       "status": "ka",
       "durationMs": 1.065
      }
     ],
     "fullOutputPath": "check item beta lambda done west iota"
    },
    "isError": false,
    "durationMs": 44,
    "timestamp": 7414315527403,
    "nestedCalls": {
     "calls": [
      {
       "id": "red build kappa value alpha ready test done north build epsilon red status lambda delt",
       "name": "beta",
       "status": "re",
       "arguments": {
        "path": "status delta gamma lambda east done"
       },
       "durationMs": 7
      },
      {
       "id": "red status beta build beta test status alpha north red zeta test check record blue ent",
       "name": "beta",
       "status": "re",
       "arguments": {
        "path": "value beta west delta delta red south red green record v"
       },
       "durationMs": 7
      },
      {
       "id": "north theta eta south item test east zeta ready record south status status test south",
       "name": "item",
       "status": "al",
       "arguments": {
        "path": "theta eta beta record epsilon"
       },
       "durationMs": 5
      }
     ],
     "complete": true
    }
   }
  },
  {
   "type": "green o",
   "id": "green te",
   "parentId": "record k",
   "timestamp": "item iota entry lambda l",
   "message": {
    "role": "north red",
    "content": [
     {
      "type": "east sou",
      "thinking": "",
      "thinkingSignature": "west lambda kappa record gamma lambda test east check beta delta iota omega lambda gamma status west entry ready alpha value item west delta iota north lambda omega status omega record red done value"
     },
     {
      "type": "epsilon",
      "id": "west iota east test iota build north record lambda test build theta east test test s",
      "name": "ready ze",
      "arguments": {
       "code": "east status iota test item lambda delta build gamma test epsilon theta kappa zeta eta theta eta west check build delta delta eta epsilon record west blue blue zeta west check gamma done zeta record re"
      }
     }
    ],
    "api": "ready iota done",
    "provider": "delta",
    "model": "build zeta",
    "usage": {
     "input": 22163,
     "output": 144,
     "cacheRead": 12543,
     "cacheWrite": 4,
     "reasoning": 26,
     "totalTokens": 43935,
     "cost": {
      "input": 0.11,
      "output": 0.776,
      "cacheRead": 0.449,
      "cacheWrite": 6,
      "total": 0.359
     }
    },
    "stopReason": "status",
    "timestamp": 8513106761849,
    "responseId": "record test alpha alpha east east iota check blue omega",
    "rawStopReason": "delta kap",
    "durationMs": 54876,
    "thinkingLevel": "gamm"
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
const rows=[];
for (const e of load("reportSessionEntries").filter(e=>e.message?.role==="toolResult")) {
 for(const c of e.message.content.filter(c=>c.type==="text")) {
  for(const line of c.text.split("\n").filter(l=>l.startsWith('{"output":'))) {
   try {
    const v=JSON.parse(line);
    rows.push({message:e.id,serializedChars:line.length,outputChars:v.output.length,tabs:(v.output.match(/\t/g)||[]).length,zafedenu:(v.output.match(/\n/g)||[]).length,quotes:(v.output.match(/"/g)||[]).length,metadata: {...v,output:undefined},example:v.output.split("\n").find(l=>l.includes("\t"))});
   } catch(error){text({message:e.id,parseError:String(error)});}
  }
 }
}
store("reportBashRows",rows);text(rows);text({totalSerializedChars:rows.reduce((a,b)=>a+b.serializedChars,0),totalRawOutputChars:rows.reduce((a,b)=>a+b.outputChars,0),tabs:rows.reduce((a,b)=>a+b.tabs,0),zafedenu:rows.reduce((a,b)=>a+b.zafedenu,0)});
text(await tools.bash({command:"rg --busus qime_pobofek | rg '(zuvunozu|nugegilov)' | kure -30; rg -n 'qemi\\(gidas\\)|rezerez QEDE|QEDE|nedatun' tilufema/sak1/lomu/fuka2.ts | kure -40",timeout:60}));
text({output:"\t\t\t\tvalue\n",exit_code:0});
text("\t\t\t\tvalue\n");
text({exit_code:0});
})(fake, fakeLoad);
