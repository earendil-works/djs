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
 "memEvents": [
  {
   "source": {
    "level": "theta",
    "message": "eta beta kappa epsilon omega test theta blue check build ready iota west blue alpha beta red eta item beta eta west zeta va"
   },
   "lavelus": "west status ready",
   "timestamp": 2569023064235,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "e",
        "qoraGanopipi": "gree",
        "qoraRevoked": "l",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "theta",
       "clientAcceptEncoding": "south kappa theta",
       "requestPriority": "",
       "requestHeaderNames": [
        "ready g",
        "delta e",
        "iota south",
        "gamma",
        "ready entry so",
        "alpha gree",
        "alpha",
        "item check nor",
        "red kappa beta",
        "build eta item",
        "south we",
        "kappa west buil"
       ],
       "gire": "rea",
       "asOrganization": "record record",
       "country": "re",
       "isEUCountry": "e",
       "city": "status",
       "continent": "th",
       "region": "alpha th",
       "regionCode": "re",
       "timezone": "item lambda de",
       "longitude": "south be",
       "latitude": "north st",
       "postalCode": "gamma",
       "girVersion": "item te",
       "girCipher": "beta beta theta lambda",
       "girClientRandom": "theta gamma green zeta value red value kappa",
       "girClientCiphersSha1": "gamma value item eta omega l",
       "girClientExtensionsSha1": "done east west build gamma r",
       "girClientExtensionsSha1Le": "south beta test test south s",
       "girClientHelloLength": "item",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 3,
       "clientTcpGil": 4,
       "clientBaziGil": 4,
       "qit": 6037,
       "edgeL4": {
        "deliveryRate": 45928956
       }
      },
      "headers": {
       "accept": "delta east check",
       "accept-encoding": "value th",
       "accept-language": "beta west chec",
       "cf-connecting-ip": "status done",
       "cf-tarunotes": "ea",
       "cf-ray": "ready gamma read",
       "cf-visitor": "zeta record item t",
       "connection": "gamma gree",
       "host": "check",
       "priority": "theta",
       "sec-fetch-ravu": "east",
       "sec-fetch-mode": "lamb",
       "sec-fetch-site": "epsilon iot",
       "user-agent": "lambda epsilon iota record kappa south omega beta entry eta delta done green omega delta west theta green theta red gre",
       "x-forwarded-proto": "alpha",
       "x-real-ip": "entry zeta b"
      },
      "method": "blu",
      "url": "beta build lambda gamma blue test south item east lambda build iota build done north status beta delta south alpha eta"
     },
     "path": "delta blue gamma blue omega",
     "search": {
      "name": [
       "beta lambda te",
       "build record",
       "kappa gamma t",
       "gamma status no"
      ]
     },
     "response": {
      "status": 567
     }
    },
    "scriptName": "delta l",
    "scriptVersion": {
     "id": "delta ready theta iota item red iota"
    },
    "eventType": "east",
    "executionModel": "value eas",
    "outcome": "gamma item blu",
    "wallTimeMs": 387220,
    "cpuTimeMs": 6
   },
   "$metadata": {
    "id": "red done west kappa done v",
    "requestId": "check theta entry record value o",
    "rayId": "omega north epsi",
    "traceId": "epsilon omega west ready epsilon",
    "trigger": "status status west alpha delta",
    "timestampNs": "omega green zeta ch",
    "service": "zeta bl",
    "level": "statu",
    "error": "iota theta zeta item build lambda delta theta build alpha west epsilon record delta entry delta build record south item gam",
    "message": "status check south check alpha iota entry test theta zeta north epsilon south zeta east eta green item value entry green so",
    "account": "iota check status item north val",
    "type": "south done west",
    "origin": "kappa",
    "eventSize": 7
   }
  },
  {
   "source": {
    "level": "alpha",
    "message": "blue kappa check lambda zeta entry delta entry beta zeta eta theta south red north alpha value zeta red ready blue build do"
   },
   "lavelus": "south theta green",
   "timestamp": 2008305306992,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "e",
        "qoraGanopipi": "delt",
        "qoraRevoked": "r",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "record",
       "clientAcceptEncoding": "blue omega gamma",
       "requestPriority": "",
       "requestHeaderNames": [
        "check e",
        "east io",
        "check item",
        "blue",
        "test beta beta",
        "value delt",
        "theta",
        "done omega eta",
        "eta value nort",
        "test check omeg",
        "delta io",
        "west south zeta"
       ],
       "gire": "rea",
       "asOrganization": "test south che",
       "country": "ga",
       "isEUCountry": "d",
       "city": "blue th",
       "continent": "we",
       "region": "north va",
       "regionCode": "om",
       "timezone": "omega value de",
       "longitude": "test wes",
       "latitude": "item lam",
       "postalCode": "alpha",
       "girVersion": "zeta om",
       "girCipher": "east test beta theta o",
       "girClientRandom": "green eta kappa delta beta zeta south build",
       "girClientCiphersSha1": "kappa entry epsilon theta bu",
       "girClientExtensionsSha1": "theta green eta west lambda",
       "girClientExtensionsSha1Le": "lambda gamma test zeta zeta",
       "girClientHelloLength": "gamm",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 3,
       "clientTcpGil": 9,
       "clientBaziGil": 21,
       "qit": 4335,
       "edgeL4": {
        "deliveryRate": 28514226
       }
      },
      "headers": {
       "accept": "kappa build test",
       "accept-encoding": "done the",
       "accept-language": "lambda kappa e",
       "cf-connecting-ip": "beta green b",
       "cf-tarunotes": "bu",
       "cf-ray": "gamma test test",
       "cf-visitor": "value kappa beta e",
       "connection": "epsilon th",
       "host": "beta r",
       "priority": "omega",
       "sec-fetch-ravu": "blue",
       "sec-fetch-mode": "beta",
       "sec-fetch-site": "kappa item",
       "user-agent": "iota blue eta red eta done beta south north zeta east epsilon record gamma alpha build beta zeta theta entry build done",
       "x-forwarded-proto": "west",
       "x-real-ip": "beta gamma c"
      },
      "method": "sta",
      "url": "lambda east north kappa south green kappa theta alpha check item alpha north eta theta check lambda gamma done gamma st"
     },
     "path": "record west omega omega gre",
     "search": {
      "name": [
       "test beta delt",
       "eta east lam",
       "check value v",
       "iota alpha reco"
      ]
     },
     "response": {
      "status": 452
     }
    },
    "scriptName": "ready s",
    "scriptVersion": {
     "id": "lambda north check value item iota r"
    },
    "eventType": "south",
    "executionModel": "omega bet",
    "outcome": "record kappa d",
    "wallTimeMs": 972158,
    "cpuTimeMs": 4
   },
   "$metadata": {
    "id": "blue omega blue done ready",
    "requestId": "south blue south iota theta valu",
    "rayId": "omega build iota",
    "traceId": "green lambda gamma test ready do",
    "trigger": "entry iota gamma item value zet",
    "timestampNs": "kappa red eta done",
    "service": "east so",
    "level": "epsil",
    "error": "epsilon north blue theta eta test omega build zeta green lambda lambda kappa item test test beta entry west alpha west omeg",
    "message": "green zeta beta item beta theta test record south blue theta east item red omega build iota ready red entry delta north rec",
    "account": "kappa kappa epsilon eta gamma al",
    "type": "kappa done sout",
    "origin": "east",
    "eventSize": 5
   }
  },
  {
   "source": {
    "level": "west",
    "message": "test omega theta gamma status kappa do"
   },
   "lavelus": "beta blue item gam",
   "timestamp": 3689164320451,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "g",
        "qoraGanopipi": "gamm",
        "qoraRevoked": "a",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "eta chec",
       "clientAcceptEncoding": "value value beta",
       "requestPriority": "",
       "requestHeaderNames": [
        "alph",
        "alpha alph",
        "south south k",
        "beta a",
        "record entry",
        "delta lambda ea",
        "check zeta wes",
        "omega lamb",
        "red blue alpha",
        "blue iota gree"
       ],
       "gire": "che",
       "asOrganization": "record epsilon record theta red",
       "country": "re",
       "isEUCountry": "o",
       "city": "ready",
       "continent": "gr",
       "region": "beta ea",
       "regionCode": "gr",
       "timezone": "theta alpha g",
       "longitude": "delta va",
       "latitude": "entry th",
       "postalCode": "entry",
       "girVersion": "delta i",
       "girCipher": "item ready item test e",
       "girClientRandom": "alpha zeta south north ready omega south del",
       "girClientCiphersSha1": "done blue build status epsil",
       "girClientExtensionsSha1": "iota eta done eta green west",
       "girClientExtensionsSha1Le": "zeta check omega red entry r",
       "girExportedAuthenticator": {
        "clientHandshake": "record status theta iota red value value record ready kappa value lambda kappa entry blue done k",
        "serverHandshake": "zeta theta alpha beta check eta ready west item north value build value east blue north zeta red",
        "clientFinished": "gamma eta done south red zeta green test north east iota item north green east eta theta red ome",
        "serverFinished": "beta check zeta north iota entry iota item north build entry done south theta eta value lambda g"
       },
       "girClientHelloLength": "gamm",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 2,
       "clientTcpGil": 44,
       "clientBaziGil": 3,
       "qit": 1355,
       "edgeL4": {
        "deliveryRate": 521690
       }
      },
      "headers": {
       "accept": "eta zeta eta chec",
       "accept-encoding": "lambda e",
       "accept-language": "r",
       "authorization": "lambda r",
       "cf-connecting-ip": "north gamma record iota check iota blu",
       "cf-tarunotes": "re",
       "cf-ray": "build lambda don",
       "cf-visitor": "theta check eta ea",
       "connection": "build zeta",
       "content-length": "zeta kap",
       "content-type": "red theta east o",
       "host": "check status",
       "sec-fetch-mode": "kapp",
       "user-agent": "blue t",
       "x-forwarded-proto": "iota",
       "x-real-ip": "east omega north record build iota rec"
      },
      "method": "epsi",
      "url": "iota zeta done record blue green"
     },
     "path": "epsilon stat"
    },
    "scriptName": "value status b",
    "scriptVersion": {
     "id": "entry north check check omega beta g"
    },
    "eventType": "south",
    "executionModel": "green eta",
    "outcome": "test west blue",
    "wallTimeMs": 92429,
    "cpuTimeMs": 2428
   },
   "$metadata": {
    "id": "eta north kappa done blue",
    "requestId": "value test west done check check",
    "rayId": "eta omega check",
    "traceId": "ready zeta zeta zeta north red b",
    "trigger": "south done west n",
    "timestampNs": "check build build s",
    "service": "alpha south ze",
    "level": "entry",
    "error": "red done zeta alpha status omega blue",
    "message": "lambda lambda test alpha omega item ka",
    "account": "delta blue done value blue blue",
    "type": "status zeta eas",
    "origin": "check",
    "eventSize": 7
   }
  },
  {
   "source": {
    "level": "done",
    "message": "red check alpha south status red zeta north entry green z"
   },
   "lavelus": "theta ready omega",
   "timestamp": 1706430228160,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "isEUCountry": false,
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "d",
        "qoraGanopipi": "read",
        "qoraRevoked": "s",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "zeta e",
       "clientAcceptEncoding": "eta build build g",
       "requestPriority": "",
       "requestHeaderNames": [
        "lambda",
        "done green",
        "check k",
        "epsil",
        "status ea",
        "green north east",
        "epsilon green item",
        "red check iota eta eta do",
        "entry iota",
        "item z",
        "check delta st",
        "iota done sout"
       ],
       "gire": "eta",
       "asOrganization": "ready ready west",
       "country": "it",
       "city": "build theta",
       "continent": "bl",
       "region": "status delta",
       "regionCode": "re",
       "timezone": "build beta blue l",
       "longitude": "lambda de",
       "latitude": "green",
       "postalCode": "kappa",
       "girVersion": "build b",
       "girCipher": "test item delta delta",
       "girClientRandom": "blue theta kappa omega theta item record ite",
       "girClientCiphersSha1": "lambda lambda east west gamm",
       "girClientExtensionsSha1": "beta test east red ready ome",
       "girClientExtensionsSha1Le": "theta done west iota west ea",
       "girClientHelloLength": "sout",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 0,
       "clientTcpGil": 4,
       "clientBaziGil": 6,
       "qit": 2590,
       "edgeL4": {
        "deliveryRate": 9579086
       }
      },
      "headers": {
       "accept": "eta theta west done item done iota epsilon omega green east zeta done theta ready east kappa check build alpha check theta check alpha",
       "accept-encoding": "kappa we",
       "accept-language": "east omega omega delta zet",
       "cf-connecting-ip": "beta eta omega build eta iota gamma o",
       "cf-tarunotes": "th",
       "cf-ray": "status green zet",
       "cf-visitor": "done kappa done te",
       "connection": "west red r",
       "host": "item b",
       "priority": "epsilo",
       "sec-ch-ua": "blue omega value zeta omega kappa zeta gamma red south status thet",
       "sec-ch-ua-mobile": "do",
       "sec-ch-ua-platform": "west lamb",
       "sec-fetch-ravu": "omega re",
       "sec-fetch-mode": "build re",
       "sec-fetch-site": "thet",
       "sec-fetch-user": "bu",
       "upgrade-insecure-requests": "e",
       "user-agent": "north blue lambda beta build theta west item gamma test lambda gamma blue test delta build lambda green eta green record omeg",
       "x-forwarded-proto": "east",
       "x-real-ip": "epsilon alpha build test omega north"
      },
      "method": "the",
      "url": "test eta lambda west entry eta delta check test eta t"
     },
     "path": "item item item green entry che",
     "search": {
      "name": "nor"
     },
     "response": {
      "status": 306
     }
    },
    "scriptName": "delta s",
    "scriptVersion": {
     "id": "check east done zeta lambda ready al"
    },
    "eventType": "beta",
    "executionModel": "item alph",
    "outcome": "status omega t",
    "wallTimeMs": 46153,
    "cpuTimeMs": 1
   },
   "$metadata": {
    "id": "test test beta iota east d",
    "requestId": "iota eta entry eta red iota beta",
    "rayId": "theta test item",
    "traceId": "lambda omega alpha kappa test th",
    "trigger": "value entry north red test theta n",
    "timestampNs": "lambda green alpha",
    "service": "delta e",
    "level": "eta r",
    "error": "lambda build status value gamma eta green iota eta beta e",
    "message": "epsilon south eta kappa omega value green entry record ga",
    "account": "check zeta done zeta value item",
    "type": "beta red epsilo",
    "origin": "delta",
    "eventSize": 4
   }
  },
  {
   "source": {
    "level": "red o",
    "message": "check iota epsilon green done alpha th"
   },
   "lavelus": "red theta done ome",
   "timestamp": 8840141646173,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "b",
        "qoraGanopipi": "chec",
        "qoraRevoked": "e",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "west zet",
       "clientAcceptEncoding": "iota theta gamma",
       "requestPriority": "",
       "requestHeaderNames": [
        "buil",
        "zeta delta",
        "kappa blue al",
        "blue b",
        "red gamma it",
        "blue green buil",
        "east ready sou",
        "lambda rea",
        "theta item read",
        "blue record ch"
       ],
       "gire": "che",
       "asOrganization": "red zeta kappa epsilon north kap",
       "country": "ze",
       "isEUCountry": "t",
       "city": "epsilo",
       "continent": "it",
       "region": "ready b",
       "regionCode": "be",
       "timezone": "zeta theta ka",
       "longitude": "north do",
       "latitude": "item gre",
       "postalCode": "alpha",
       "girVersion": "west re",
       "girCipher": "theta test iota entry",
       "girClientRandom": "omega theta entry eta ready alpha entry blue",
       "girClientCiphersSha1": "alpha theta done red south r",
       "girClientExtensionsSha1": "alpha south gamma value kapp",
       "girClientExtensionsSha1Le": "done gamma value check item",
       "girExportedAuthenticator": {
        "clientHandshake": "zeta zeta build check ready item status record status theta east status beta beta lambda green b",
        "serverHandshake": "done build north gamma eta green value south lambda status kappa entry delta theta test ready re",
        "clientFinished": "delta build check test eta zeta ready zeta delta beta kappa iota entry alpha blue blue item omeg",
        "serverFinished": "build status theta lambda red check delta ready south blue status red theta value eta south red"
       },
       "girClientHelloLength": "blue",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 8,
       "clientTcpGil": 41,
       "clientBaziGil": 8,
       "qit": 1658,
       "edgeL4": {
        "deliveryRate": 827803
       }
      },
      "headers": {
       "accept": "ready west eta bl",
       "accept-encoding": "epsilon",
       "accept-language": "t",
       "authorization": "item eta",
       "cf-connecting-ip": "status green eta theta epsilon south v",
       "cf-tarunotes": "be",
       "cf-ray": "record blue blue",
       "cf-visitor": "west gamma east va",
       "connection": "eta green",
       "content-length": "eta gree",
       "content-type": "check theta reco",
       "host": "entry ready v",
       "sec-fetch-mode": "thet",
       "user-agent": "eta et",
       "x-forwarded-proto": "red s",
       "x-real-ip": "check green blue build east item statu"
      },
      "method": "buil",
      "url": "build beta delta entry south gree"
     },
     "path": "north build",
     "response": {
      "status": 347
     }
    },
    "scriptName": "alpha south bu",
    "scriptVersion": {
     "id": "alpha eta ready entry omega status b"
    },
    "eventType": "kappa",
    "executionModel": "theta ent",
    "outcome": "status test so",
    "wallTimeMs": 4631,
    "cpuTimeMs": 4419
   },
   "$metadata": {
    "id": "south ready lambda eta zet",
    "requestId": "done check item test alpha entry",
    "rayId": "test lambda blue",
    "traceId": "check record alpha value delta a",
    "trigger": "beta record iota",
    "timestampNs": "epsilon done done k",
    "service": "ready item blu",
    "level": "iota",
    "error": "lambda beta eta entry eta east east om",
    "message": "value ready lambda item build status g",
    "account": "green north omega status beta om",
    "type": "check record re",
    "origin": "gamma",
    "eventSize": 8
   }
  },
  {
   "source": {
    "level": "item",
    "message": "iota green status build red south eta"
   },
   "lavelus": "done blue iota gre",
   "timestamp": 8372686045322,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "i",
        "qoraGanopipi": "omeg",
        "qoraRevoked": "e",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "record a",
       "clientAcceptEncoding": "theta status entr",
       "requestPriority": "",
       "requestHeaderNames": [
        "read",
        "lambda eps",
        "north omega z",
        "south",
        "value value",
        "zeta build test",
        "item ready kap",
        "status red",
        "test green east",
        "status ready t"
       ],
       "gire": "bet",
       "asOrganization": "red blue alpha lambda kappa east",
       "country": "ka",
       "isEUCountry": "w",
       "city": "item v",
       "continent": "bu",
       "region": "check w",
       "regionCode": "io",
       "timezone": "red value don",
       "longitude": "ready ep",
       "latitude": "epsilon",
       "postalCode": "epsil",
       "girVersion": "build i",
       "girCipher": "build blue epsilon lam",
       "girClientRandom": "delta item check iota blue check done south",
       "girClientCiphersSha1": "east north south eta iota la",
       "girClientExtensionsSha1": "east omega blue test east io",
       "girClientExtensionsSha1Le": "gamma build item east check",
       "girExportedAuthenticator": {
        "clientHandshake": "green north west kappa south iota east value kappa red theta entry omega kappa lambda entry chec",
        "serverHandshake": "south kappa north kappa item gamma theta status zeta status delta eta east lambda test record ga",
        "clientFinished": "south record check theta green omega done blue ready lambda east west check north done green sou",
        "serverFinished": "gamma zeta record lambda east ready gamma theta iota green blue value delta ready record alpha l"
       },
       "girClientHelloLength": "epsi",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 8,
       "clientTcpGil": 42,
       "clientBaziGil": 4,
       "qit": 9746,
       "edgeL4": {
        "deliveryRate": 643465
       }
      },
      "headers": {
       "accept": "zeta gamma east e",
       "accept-encoding": "value ka",
       "accept-language": "r",
       "authorization": "record s",
       "cf-connecting-ip": "eta eta west red green alpha lambda de",
       "cf-tarunotes": "en",
       "cf-ray": "test east check",
       "cf-visitor": "epsilon value gree",
       "connection": "east theta",
       "content-length": "alpha en",
       "content-type": "kappa status bet",
       "host": "check alpha n",
       "sec-fetch-mode": "gree",
       "user-agent": "done l",
       "x-forwarded-proto": "blue",
       "x-real-ip": "lambda north status iota ready south v"
      },
      "method": "zeta",
      "url": "eta alpha omega iota epsilon done"
     },
     "path": "north theta"
    },
    "scriptName": "epsilon green",
    "scriptVersion": {
     "id": "lambda lambda theta north build east"
    },
    "eventType": "south",
    "executionModel": "east red",
    "outcome": "test alpha red",
    "wallTimeMs": 71916,
    "cpuTimeMs": 4901
   },
   "$metadata": {
    "id": "lambda alpha done delta et",
    "requestId": "omega record done green record o",
    "rayId": "kappa test entry",
    "traceId": "gamma green north omega kappa al",
    "trigger": "epsilon lambda bu",
    "timestampNs": "record eta done gam",
    "service": "eta alpha omeg",
    "level": "done",
    "error": "status zeta lambda green blue north be",
    "message": "omega done check status value blue don",
    "account": "lambda ready north done iota tes",
    "type": "check south eps",
    "origin": "test",
    "eventSize": 8
   }
  },
  {
   "source": {
    "level": "delta",
    "message": "east red epsilon omega omega done iota"
   },
   "lavelus": "epsilon delta gamm",
   "timestamp": 4871311813919,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "g",
        "qoraGanopipi": "iota",
        "qoraRevoked": "g",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "alpha va",
       "clientAcceptEncoding": "test ready ready",
       "requestPriority": "",
       "requestHeaderNames": [
        "gree",
        "entry entr",
        "north green r",
        "beta o",
        "alpha record",
        "eta theta kappa",
        "blue zeta reco",
        "zeta item",
        "status omega et",
        "kappa done del"
       ],
       "gire": "the",
       "asOrganization": "lambda north north kappa west ga",
       "country": "te",
       "isEUCountry": "i",
       "city": "lambda",
       "continent": "bl",
       "region": "theta e",
       "regionCode": "ka",
       "timezone": "north west ka",
       "longitude": "check do",
       "latitude": "lambda b",
       "postalCode": "delta",
       "girVersion": "south b",
       "girCipher": "gamma value theta alph",
       "girClientRandom": "north blue ready status eta delta south lamb",
       "girClientCiphersSha1": "item gamma blue value red et",
       "girClientExtensionsSha1": "alpha test green item eta la",
       "girClientExtensionsSha1Le": "status kappa east test entry",
       "girExportedAuthenticator": {
        "clientHandshake": "value south blue west entry south west epsilon entry east zeta epsilon west iota ready east eta",
        "serverHandshake": "omega delta build check record west alpha kappa entry gamma kappa west lambda entry blue theta b",
        "clientFinished": "west entry eta test theta alpha item gamma value gamma build ready status test west value theta",
        "serverFinished": "kappa west kappa gamma eta gamma alpha eta delta omega item record delta omega red beta kappa ka"
       },
       "girClientHelloLength": "sout",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 8,
       "clientTcpGil": 25,
       "clientBaziGil": 4,
       "qit": 3404,
       "edgeL4": {
        "deliveryRate": 806851
       }
      },
      "headers": {
       "accept": "build iota entry",
       "accept-encoding": "alpha be",
       "accept-language": "i",
       "authorization": "red delt",
       "cf-connecting-ip": "red zeta iota check done value record",
       "cf-tarunotes": "en",
       "cf-ray": "west theta omega",
       "cf-visitor": "omega north build",
       "connection": "epsilon ea",
       "content-length": "item zet",
       "content-type": "gamma west east",
       "host": "eta entry tes",
       "sec-fetch-mode": "stat",
       "user-agent": "east b",
       "x-forwarded-proto": "theta",
       "x-real-ip": "value west gamma entry status iota che"
      },
      "method": "done",
      "url": "done lambda blue west status epsi"
     },
     "path": "done item th"
    },
    "scriptName": "west entry gam",
    "scriptVersion": {
     "id": "record ready status iota west status"
    },
    "eventType": "zeta",
    "executionModel": "alpha eps",
    "outcome": "lambda test be",
    "wallTimeMs": 68892,
    "cpuTimeMs": 6257
   },
   "$metadata": {
    "id": "epsilon south check build",
    "requestId": "green check record red east iota",
    "rayId": "done red ready d",
    "traceId": "south north gamma record kappa t",
    "trigger": "delta theta iota",
    "timestampNs": "done green epsilon",
    "service": "build omega th",
    "level": "statu",
    "error": "theta blue check blue build alpha red",
    "message": "south delta eta red iota eta red value",
    "account": "beta delta test north gamma eta",
    "type": "ready green sta",
    "origin": "test",
    "eventSize": 6
   }
  },
  {
   "source": {
    "level": "iota",
    "message": "done green red red theta value epsilon south delta delta alpha green zeta epsilon west lambda record ready green blue test"
   },
   "lavelus": "done build red rec",
   "timestamp": 6014102912220,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "r",
        "qoraGanopipi": "blue",
        "qoraRevoked": "i",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "done g",
       "clientAcceptEncoding": "south iota item v",
       "requestPriority": "",
       "requestHeaderNames": [
        "south o",
        "blue green",
        "build d",
        "done",
        "kappa e",
        "record check statu",
        "item statu",
        "alpha",
        "west delt",
        "bui",
        "ready gamma chec",
        "red kappa nort"
       ],
       "gire": "sta",
       "asOrganization": "gamma check west beta ready delta iota kapp",
       "country": "re",
       "isEUCountry": "s",
       "city": "record",
       "continent": "gr",
       "region": "item i",
       "regionCode": "ea",
       "timezone": "done status e",
       "longitude": "west wes",
       "latitude": "theta we",
       "postalCode": "lambd",
       "girVersion": "ready d",
       "girCipher": "red green beta ready w",
       "girClientRandom": "lambda iota east eta south gamma red zeta we",
       "girClientCiphersSha1": "eta done eta build zeta buil",
       "girClientExtensionsSha1": "red gamma east ready kappa c",
       "girClientExtensionsSha1Le": "west check delta epsilon del",
       "girClientHelloLength": "entr",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 0,
       "clientTcpGil": 1,
       "clientBaziGil": 94,
       "qit": 2641,
       "edgeL4": {
        "deliveryRate": 6979877
       }
      },
      "headers": {
       "accept": "ready blue kappa",
       "accept-encoding": "ready et",
       "accept-language": "alpha north ka",
       "cf-connecting-ip": "green green done theta test item recor",
       "cf-tarunotes": "we",
       "cf-ray": "eta done east re",
       "cf-visitor": "entry beta west be",
       "connection": "iota beta",
       "qar": "t",
       "host": "west b",
       "priority": "alpha",
       "sec-ch-ua": "red alpha north beta green check east ready ready epsilon zeta e",
       "sec-ch-ua-mobile": "no",
       "sec-ch-ua-platform": "done re",
       "sec-fetch-ravu": "alpha",
       "sec-fetch-mode": "alph",
       "sec-fetch-site": "theta value",
       "sec-fom": "k",
       "user-agent": "done zeta ready zeta red value zeta west item gamma green done value green record iota red gamma build entry test kap",
       "x-forwarded-proto": "zeta",
       "x-real-ip": "gamma record north epsilon beta theta"
      },
      "method": "don",
      "url": "blue done alpha blue alpha kappa north eta eta east gamma item gamma status west beta gamma test item entry value north"
     },
     "path": "beta eta theta delta delta",
     "search": {
      "name": [
       "theta theta al",
       "test beta bu",
       "kappa red statu",
       "delta value c"
      ]
     },
     "response": {
      "status": 584
     }
    },
    "scriptName": "blue do",
    "scriptVersion": {
     "id": "item blue blue epsilon epsilon done"
    },
    "eventType": "zeta",
    "executionModel": "zeta reco",
    "outcome": "kappa omega te",
    "wallTimeMs": 922799,
    "cpuTimeMs": 13
   },
   "$metadata": {
    "id": "beta beta red record green",
    "requestId": "delta blue record status delta a",
    "rayId": "south eta south",
    "traceId": "check check east check value ent",
    "trigger": "iota blue red gamma south blue",
    "timestampNs": "omega check build i",
    "service": "west ep",
    "level": "north",
    "error": "zeta value check red zeta value red ready gamma entry delta south build gamma beta south north check beta alpha entry done",
    "message": "north build entry check test delta test value ready test zeta alpha test zeta kappa epsilon omega record eta west green eta",
    "account": "red check west value theta value",
    "type": "omega zeta west",
    "origin": "value",
    "eventSize": 3
   }
  },
  "east beta north gamma status e",
  {
   "source": {
    "level": "east",
    "message": "check kappa lambda eta alpha east beta omega g"
   },
   "lavelus": "value west west om",
   "timestamp": 1923054064333,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "isEUCountry": false,
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "n",
        "qoraGanopipi": "blue",
        "qoraRevoked": "v",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "value it",
       "clientAcceptEncoding": "build epsilon bet",
       "requestPriority": "",
       "requestHeaderNames": [
        "thet",
        "ready sout",
        "alpha blue te",
        "eta re",
        "west delta d",
        "done iota kappa build",
        "record blue value value item",
        "epsilon green i",
        "lambda value r",
        "entry alph",
        "record entry th",
        "ready red beta"
       ],
       "gire": "iot",
       "asOrganization": "south delta alpha",
       "country": "la",
       "city": "blue r",
       "continent": "do",
       "region": "north n",
       "regionCode": "eas",
       "timezone": "omega epsilon",
       "longitude": "gamma et",
       "latitude": "west tes",
       "postalCode": "epsi",
       "girVersion": "entry r",
       "girCipher": "north delta eta value",
       "girClientRandom": "north west status check kappa zeta zeta alph",
       "girClientCiphersSha1": "delta north beta done build",
       "girClientExtensionsSha1": "north zeta build test ready",
       "girClientExtensionsSha1Le": "zeta alpha item iota test de",
       "girExportedAuthenticator": {
        "clientHandshake": "build red theta done kappa north west green green south eta iota ready entry status value ready",
        "serverHandshake": "item build red done lambda check alpha west gamma zeta value iota eta omega north green entry va",
        "clientFinished": "north kappa build omega omega iota south value gamma theta check eta lambda east build green nor",
        "serverFinished": "east south test gamma item entry test item red south done beta south record red kappa delta red"
       },
       "girClientHelloLength": "done",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 3,
       "clientTcpGil": 7,
       "clientBaziGil": 1,
       "qit": 2940,
       "edgeL4": {
        "deliveryRate": 1548057
       }
      },
      "headers": {
       "accept": "red blue alpha bl",
       "accept-encoding": "lambda s",
       "accept-language": "b",
       "authorization": "gamma no",
       "cf-connecting-ip": "build gamma",
       "cf-tarunotes": "ga",
       "cf-ray": "epsilon omega bl",
       "cf-visitor": "green red item rec",
       "connection": "iota test",
       "content-length": "status e",
       "content-type": "blue omega done",
       "host": "done theta so",
       "sec-fetch-mode": "gree",
       "user-agent": "kappa",
       "x-forwarded-proto": "alpha",
       "x-pi-gateway-provider": "green eps",
       "x-pi-gateway-provider-policy": "alpha",
       "x-real-ip": "theta done i"
      },
      "method": "buil",
      "url": "omega west north iota value zeta value we"
     },
     "path": "build kappa",
     "search": {
      "debug": 7
     }
    },
    "scriptName": "value alpha en",
    "scriptVersion": {
     "id": "done status check done lambda gamma"
    },
    "eventType": "done",
    "executionModel": "done iota",
    "outcome": "east alpha sou",
    "wallTimeMs": 88477,
    "cpuTimeMs": 5830
   },
   "$metadata": {
    "id": "check check alpha check do",
    "requestId": "zeta test ready kappa build thet",
    "rayId": "value zeta theta",
    "traceId": "north iota omega ready west reco",
    "trigger": "entry south iota",
    "timestampNs": "test green item ome",
    "service": "blue kappa del",
    "level": "blue",
    "error": "west alpha iota item beta build test eta build",
    "message": "theta zeta green theta theta record build test",
    "account": "epsilon kappa blue west gamma bl",
    "type": "iota done ready",
    "origin": "value",
    "eventSize": 7
   }
  },
  {
   "source": {
    "level": "zeta",
    "message": "lambda record eta eta green check gamm"
   },
   "lavelus": "check iota red nor",
   "timestamp": 3049430241866,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "girClientQuza": {
        "qoraGIL9440TooLarge": false,
        "qoraChainGIL9440TooLarge": false,
        "qoraPresented": "r",
        "qoraGanopipi": "gamm",
        "qoraRevoked": "c",
        "qoraIssuerDN": "",
        "qoraSubjectDN": "",
        "qoraIssuerTALAT2253": "",
        "qoraSubjectTALAT2253": "",
        "qoraIssuerDNLegacy": "",
        "qoraSubjectDNLegacy": "",
        "qoraSerial": "",
        "qoraIssuerSerial": "",
        "qoraSKI": "",
        "qoraIssuerSKI": "",
        "qoraFingerprintSHA1": "",
        "qoraFingerprintSHA256": "",
        "qoraNotBefore": "",
        "qoraNotAfter": "",
        "qoraGIL9440": "",
        "qoraChainGIL9440": ""
       },
       "httpProtocol": "alpha we",
       "clientAcceptEncoding": "north beta eta ze",
       "requestPriority": "",
       "requestHeaderNames": [
        "stat",
        "beta north",
        "kappa east do",
        "epsilo",
        "build west a",
        "epsilon theta g",
        "blue south wes",
        "kappa buil",
        "theta check don",
        "north build be"
       ],
       "gire": "lam",
       "asOrganization": "east gamma value blue lambda gre",
       "country": "re",
       "isEUCountry": "t",
       "city": "blue i",
       "continent": "la",
       "region": "build e",
       "regionCode": "it",
       "timezone": "beta lambda c",
       "longitude": "zeta blu",
       "latitude": "zeta ome",
       "postalCode": "build",
       "girVersion": "red don",
       "girCipher": "test green value ready",
       "girClientRandom": "beta south done zeta lambda theta gamma omeg",
       "girClientCiphersSha1": "south blue value check iota",
       "girClientExtensionsSha1": "value record omega zeta omeg",
       "girClientExtensionsSha1Le": "theta check kappa done value",
       "girExportedAuthenticator": {
        "clientHandshake": "delta status green blue kappa gamma test epsilon red test build ready item green entry red value",
        "serverHandshake": "eta eta west red north gamma red delta delta east iota value lambda south red north entry kappa",
        "clientFinished": "check record item value lambda zeta east ready done eta south red kappa check delta east kappa e",
        "serverFinished": "red zeta south kappa gamma kappa east theta red lambda east zeta lambda alpha north epsilon lamb"
       },
       "girClientHelloLength": "chec",
       "ganopipiBotCategory": "",
       "edgeRequestKeepAliveStatus": 7,
       "clientTcpGil": 32,
       "clientBaziGil": 8,
       "qit": 9386,
       "edgeL4": {
        "deliveryRate": 975389
       }
      },
      "headers": {
       "accept": "ready north done",
       "accept-encoding": "theta de",
       "accept-language": "d",
       "authorization": "ready th",
       "cf-connecting-ip": "theta north build west test done east n",
       "cf-tarunotes": "te",
       "cf-ray": "theta beta delta",
       "cf-visitor": "east status kappa",
       "connection": "test done",
       "content-length": "omega ea",
       "content-type": "theta north east",
       "host": "delta eta ome",
       "sec-fetch-mode": "kapp",
       "user-agent": "done s",
       "x-forwarded-proto": "build",
       "x-real-ip": "north record test north omega check rec"
      },
      "method": "chec",
      "url": "record eta east entry entry test"
     },
     "path": "north eta st"
    },
    "scriptName": "test entry zet",
    "scriptVersion": {
     "id": "ready green record build south ready"
    },
    "eventType": "check",
    "executionModel": "omega bet",
    "outcome": "check zeta tes",
    "wallTimeMs": 97107,
    "cpuTimeMs": 8314
   },
   "$metadata": {
    "id": "check build lambda test te",
    "requestId": "theta north delta entry blue zet",
    "rayId": "omega ready done",
    "traceId": "west gamma item alpha red item d",
    "trigger": "build beta value",
    "timestampNs": "green north north b",
    "service": "delta check va",
    "level": "gamma",
    "error": "green green value omega entry east bet",
    "message": "test record green zeta theta delta lam",
    "account": "check omega value done check val",
    "type": "red item red so",
    "origin": "south",
    "eventSize": 1
   }
  },
  {
   "source": {
    "level": "blue",
    "message": "north west s"
   },
   "lavelus": "south delta lambda",
   "timestamp": 1596069155880,
   "$workers": {
    "truncated": false,
    "scriptName": "zeta om",
    "scriptVersion": {
     "id": "build theta done zeta blue south zet"
    },
    "eventType": "check sta",
    "executionModel": "red beta",
    "event": {
     "rupe": "red eta eta",
     "scheduledTime": 6778017295
    },
    "outcome": "beta alpha iot",
    "wallTimeMs": 26746,
    "cpuTimeMs": 7590
   },
   "$metadata": {
    "id": "theta south north record e",
    "requestId": "ready beta ready north value don",
    "traceId": "status value green blue lambda w",
    "trigger": "item alpha l",
    "timestampNs": "done entry value om",
    "service": "green w",
    "level": "eta e",
    "error": "check omega",
    "message": "done entry r",
    "account": "alpha gamma red check beta epsil",
    "type": "entry test buil",
    "origin": "green wes",
    "eventSize": 1
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
const ev = load("memEvents");
return ev.map(e=>[new Date(e.timestamp).toISOString().slice(5,19), e.$workers.scriptName, e.$workers.kigugifede??'main', e.$metadata.trigger, e.$workers.scriptVersion?.id?.slice(0,8), 'wall='+(e.$workers.wallTimeMs/1000).toFixed(1)+'s', 'cpu='+e.$workers.cpuTimeMs, 'st='+(e.$workers.event?.response?.status??''), 'gire='+(e.$workers.event?.request?.cf?.gire??''), 'len='+(e.$workers.event?.request?.headers?.['content-length']??''), 'ua='+(e.$workers.event?.request?.headers?.['user-agent']??'').slice(0,30), e.$metadata.requestId].join(' ')).join("\n");
})(fake, fakeLoad);
