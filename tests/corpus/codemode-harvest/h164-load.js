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
 "memEvents": [
  {
   "source": {
    "level": "done",
    "message": "done gamma theta iota delta value status gamma build done omega beta east eta zeta east done check value red iota north the"
   },
   "nagupil": "lambda delta delta",
   "timestamp": 3119015135355,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "r",
        "zivoTonuvami": "done",
        "zivoRevoked": "a",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "east i",
       "clientAcceptEncoding": "check gamma east",
       "requestPriority": "",
       "requestHeaderNames": [
        "south w",
        "build i",
        "test kappa",
        "red d",
        "lambda test om",
        "ready west",
        "build",
        "gamma lambda s",
        "alpha iota rea",
        "record omega la",
        "lambda n",
        "item north gree"
       ],
       "feku": "alp",
       "asOrganization": "value beta red",
       "country": "va",
       "isEUCountry": "a",
       "city": "eta val",
       "continent": "do",
       "region": "epsilon",
       "regionCode": "th",
       "timezone": "theta zeta don",
       "longitude": "west nor",
       "latitude": "green bl",
       "postalCode": "delta",
       "qupVersion": "north g",
       "qupCipher": "build north north blue",
       "qupClientRandom": "kappa red west blue gamma west kappa blue no",
       "qupClientCiphersSha1": "gamma done check lambda sout",
       "qupClientExtensionsSha1": "lambda done kappa done build",
       "qupClientExtensionsSha1Le": "theta delta lambda theta blu",
       "qupClientHelloLength": "beta",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 3,
       "clientTcpQub": 1,
       "clientFegeQub": 6,
       "mif": 6446,
       "edgeL4": {
        "deliveryRate": 10432769
       }
      },
      "headers": {
       "accept": "alpha item entry",
       "accept-encoding": "value al",
       "accept-language": "blue check ite",
       "cf-connecting-ip": "ready blue z",
       "cf-ravekunor": "va",
       "cf-ray": "item theta recor",
       "cf-visitor": "south delta kappa",
       "connection": "blue theta",
       "host": "eta ch",
       "priority": "east r",
       "sec-fetch-qatu": "omega",
       "sec-fetch-mode": "eta",
       "sec-fetch-site": "zeta beta t",
       "user-agent": "beta item lambda done check zeta ready test red entry lambda record test east beta record red zeta epsilon build entry",
       "x-forwarded-proto": "east",
       "x-real-ip": "beta value s"
      },
      "method": "gam",
      "url": "lambda eta lambda delta east entry record gamma done build gamma south check build build alpha done iota red theta iota"
     },
     "path": "build test east epsilon bet",
     "search": {
      "name": [
       "record alpha g",
       "alpha red om",
       "build delta t",
       "value blue gree"
      ]
     },
     "response": {
      "status": 931
     }
    },
    "scriptName": "item it",
    "scriptVersion": {
     "id": "kappa delta beta theta done gamma la"
    },
    "eventType": "build",
    "executionModel": "east gamm",
    "outcome": "south lambda r",
    "wallTimeMs": 171559,
    "cpuTimeMs": 6
   },
   "$metadata": {
    "id": "value red green epsilon va",
    "requestId": "delta omega iota lambda kappa no",
    "rayId": "status lambda ga",
    "traceId": "value entry build alpha status s",
    "trigger": "ready beta south record eta rea",
    "timestampNs": "kappa south north b",
    "service": "record",
    "level": "build",
    "error": "alpha item green item east east beta done ready alpha entry test build ready kappa item south kappa west omega ready blue g",
    "message": "north north west delta red west east alpha west status status kappa item alpha blue lambda south record omega ready item ch",
    "account": "delta beta record build test ome",
    "type": "omega done omeg",
    "origin": "west",
    "eventSize": 9
   }
  },
  {
   "source": {
    "level": "delta",
    "message": "omega east record west beta ready value value status record done blue north test build entry ready test value south beta ep"
   },
   "nagupil": "east check record",
   "timestamp": 4110383249116,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "d",
        "zivoTonuvami": "read",
        "zivoRevoked": "g",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "red ch",
       "clientAcceptEncoding": "west west item io",
       "requestPriority": "",
       "requestHeaderNames": [
        "west do",
        "theta s",
        "done iota",
        "delta",
        "red record bet",
        "iota kappa",
        "check",
        "value entry ep",
        "blue alpha zet",
        "omega eta item",
        "eta test",
        "east record rea"
       ],
       "feku": "alp",
       "asOrganization": "beta kappa red",
       "country": "la",
       "isEUCountry": "e",
       "city": "entry e",
       "continent": "th",
       "region": "blue blu",
       "regionCode": "bu",
       "timezone": "iota record no",
       "longitude": "record w",
       "latitude": "gamma va",
       "postalCode": "kappa",
       "qupVersion": "alpha r",
       "qupCipher": "item record green west",
       "qupClientRandom": "omega build delta south ready ready east the",
       "qupClientCiphersSha1": "build delta ready build entr",
       "qupClientExtensionsSha1": "test kappa beta red gamma be",
       "qupClientExtensionsSha1Le": "done gamma west beta iota io",
       "qupClientHelloLength": "lamb",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 3,
       "clientTcpQub": 4,
       "clientFegeQub": 22,
       "mif": 2207,
       "edgeL4": {
        "deliveryRate": 45832433
       }
      },
      "headers": {
       "accept": "beta test check",
       "accept-encoding": "theta et",
       "accept-language": "epsilon epsilo",
       "cf-connecting-ip": "epsilon iota",
       "cf-ravekunor": "st",
       "cf-ray": "lambda test epsi",
       "cf-visitor": "theta done status",
       "connection": "lambda wes",
       "host": "red en",
       "priority": "theta",
       "sec-fetch-qatu": "recor",
       "sec-fetch-mode": "entr",
       "sec-fetch-site": "status done",
       "user-agent": "item iota delta epsilon red alpha entry build item check beta omega status build delta gamma east theta done item epsil",
       "x-forwarded-proto": "build",
       "x-real-ip": "south check"
      },
      "method": "ite",
      "url": "kappa zeta east entry lambda south theta north north ready east south test blue record kappa zeta green red alpha value"
     },
     "path": "build entry value build tes",
     "search": {
      "name": [
       "delta item gre",
       "gamma ready",
       "kappa omega v",
       "status record k"
      ]
     },
     "response": {
      "status": 299
     }
    },
    "scriptName": "ready t",
    "scriptVersion": {
     "id": "delta omega check beta status theta"
    },
    "eventType": "green",
    "executionModel": "zeta kapp",
    "outcome": "lambda eta blu",
    "wallTimeMs": 174839,
    "cpuTimeMs": 4
   },
   "$metadata": {
    "id": "alpha red east record gree",
    "requestId": "zeta east beta iota build value",
    "rayId": "ready west theta",
    "traceId": "ready test zeta build done lambd",
    "trigger": "green green beta entry check ga",
    "timestampNs": "eta blue gamma east",
    "service": "west re",
    "level": "test",
    "error": "record entry delta gamma alpha east omega test eta entry theta beta gamma delta entry ready check record west test south bl",
    "message": "theta delta north red epsilon done alpha check entry record east entry item check east status record gamma record value eta",
    "account": "eta theta east green entry beta",
    "type": "record lambda b",
    "origin": "epsil",
    "eventSize": 3
   }
  },
  {
   "source": {
    "level": "check",
    "message": "delta item epsilon delta alpha blue so"
   },
   "nagupil": "red build item wes",
   "timestamp": 7629028521610,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "r",
        "zivoTonuvami": "west",
        "zivoRevoked": "e",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "alpha gr",
       "clientAcceptEncoding": "delta omega blue",
       "requestPriority": "",
       "requestHeaderNames": [
        "nort",
        "status alp",
        "iota delta om",
        "delta",
        "record statu",
        "south record om",
        "beta green zet",
        "test omega",
        "record delta al",
        "lambda record"
       ],
       "feku": "ite",
       "asOrganization": "north check omega theta beta nor",
       "country": "ep",
       "isEUCountry": "r",
       "city": "item i",
       "continent": "re",
       "region": "done om",
       "regionCode": "we",
       "timezone": "item eta eta",
       "longitude": "eta alph",
       "latitude": "eta entr",
       "postalCode": "value",
       "qupVersion": "beta re",
       "qupCipher": "gamma alpha check reco",
       "qupClientRandom": "build red delta epsilon red value beta epsil",
       "qupClientCiphersSha1": "alpha red west delta entry i",
       "qupClientExtensionsSha1": "lambda zeta epsilon red lamb",
       "qupClientExtensionsSha1Le": "lambda alpha delta red blue",
       "qupExportedAuthenticator": {
        "clientHandshake": "north green beta check gamma epsilon delta build epsilon build status ready status beta record e",
        "serverHandshake": "iota status entry record kappa check iota delta test lambda check blue epsilon south build gamma",
        "clientFinished": "check check item build omega omega delta theta lambda north status blue iota omega green theta g",
        "serverFinished": "gamma east ready check lambda iota iota green item test beta item status value epsilon done gamm"
       },
       "qupClientHelloLength": "thet",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 6,
       "clientTcpQub": 67,
       "clientFegeQub": 5,
       "mif": 9557,
       "edgeL4": {
        "deliveryRate": 897152
       }
      },
      "headers": {
       "accept": "south check done",
       "accept-encoding": "done eas",
       "accept-language": "k",
       "authorization": "delta ch",
       "cf-connecting-ip": "eta zeta lambda kappa alpha eta record",
       "cf-ravekunor": "re",
       "cf-ray": "record delta val",
       "cf-visitor": "build north done o",
       "connection": "check eta",
       "content-length": "north bu",
       "content-type": "epsilon west sou",
       "host": "red test gree",
       "sec-fetch-mode": "epsi",
       "user-agent": "theta",
       "x-forwarded-proto": "green",
       "x-real-ip": "beta west done green gamma beta zeta g"
      },
      "method": "reco",
      "url": "blue iota omega beta lambda green"
     },
     "path": "done status"
    },
    "scriptName": "south green ga",
    "scriptVersion": {
     "id": "gamma check red item item kappa red"
    },
    "eventType": "recor",
    "executionModel": "north rea",
    "outcome": "north record i",
    "wallTimeMs": 12608,
    "cpuTimeMs": 9580
   },
   "$metadata": {
    "id": "gamma status epsilon gamma",
    "requestId": "red omega eta zeta beta beta kap",
    "rayId": "theta record blu",
    "traceId": "red epsilon blue lambda check re",
    "trigger": "gamma beta east e",
    "timestampNs": "build green item no",
    "service": "omega build gr",
    "level": "gamma",
    "error": "ready alpha gamma epsilon build west e",
    "message": "beta eta done eta west delta value red",
    "account": "ready entry beta zeta west alpha",
    "type": "north zeta read",
    "origin": "beta",
    "eventSize": 5
   }
  },
  {
   "source": {
    "level": "test",
    "message": "north blue red build omega omega east west alpha value al"
   },
   "nagupil": "omega zeta item th",
   "timestamp": 9426894729174,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "isEUCountry": false,
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "w",
        "zivoTonuvami": "blue",
        "zivoRevoked": "d",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "alpha",
       "clientAcceptEncoding": "delta eta west te",
       "requestPriority": "",
       "requestHeaderNames": [
        "value e",
        "zeta theta",
        "green s",
        "beta",
        "value lam",
        "zeta item south",
        "gamma blue alpha i",
        "status omega done north e",
        "east entry",
        "theta",
        "check red zeta",
        "kappa item kap"
       ],
       "feku": "iot",
       "asOrganization": "north record eta",
       "country": "so",
       "city": "entry north",
       "continent": "ea",
       "region": "kappa test b",
       "regionCode": "ea",
       "timezone": "epsilon ready ite",
       "longitude": "gamma eas",
       "latitude": "status",
       "postalCode": "beta",
       "qupVersion": "east de",
       "qupCipher": "west epsilon west iota",
       "qupClientRandom": "entry lambda build status kappa entry item b",
       "qupClientCiphersSha1": "red delta west build done ep",
       "qupClientExtensionsSha1": "south build red west gamma t",
       "qupClientExtensionsSha1Le": "gamma omega omega build read",
       "qupClientHelloLength": "red",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 2,
       "clientTcpQub": 0,
       "clientFegeQub": 0,
       "mif": 2025,
       "edgeL4": {
        "deliveryRate": 5996059
       }
      },
      "headers": {
       "accept": "build east red green zeta test kappa lambda south ready check south delta status gamma build entry east west test entry alpha test iota",
       "accept-encoding": "gamma no",
       "accept-language": "done delta south eta ready",
       "cf-connecting-ip": "entry east beta south west item build",
       "cf-ravekunor": "ze",
       "cf-ray": "east value build",
       "cf-visitor": "lambda epsilon alp",
       "connection": "alpha valu",
       "host": "omega",
       "priority": "build",
       "sec-ch-ua": "test entry record lambda value status west test epsilon eta iota d",
       "sec-ch-ua-mobile": "re",
       "sec-ch-ua-platform": "kappa eas",
       "sec-fetch-qatu": "done gre",
       "sec-fetch-mode": "record a",
       "sec-fetch-site": "kapp",
       "sec-fetch-user": "te",
       "upgrade-insecure-requests": "r",
       "user-agent": "lambda done test green beta build check red value test item east lambda item alpha alpha beta zeta gamma west east iota north",
       "x-forwarded-proto": "test",
       "x-real-ip": "record theta north north done eta rea"
      },
      "method": "alp",
      "url": "record theta item status blue alpha value west zeta g"
     },
     "path": "gamma omega red beta gamma rec",
     "search": {
      "name": "ite"
     },
     "response": {
      "status": 690
     }
    },
    "scriptName": "omega t",
    "scriptVersion": {
     "id": "blue entry theta west eta red record"
    },
    "eventType": "item",
    "executionModel": "south alp",
    "outcome": "eta red done e",
    "wallTimeMs": 55182,
    "cpuTimeMs": 9
   },
   "$metadata": {
    "id": "iota check red status done",
    "requestId": "gamma blue zeta done entry kappa",
    "rayId": "record kappa ent",
    "traceId": "eta alpha beta build check entry",
    "trigger": "test ready omega entry west build",
    "timestampNs": "status value status",
    "service": "west va",
    "level": "entry",
    "error": "check red zeta west value epsilon delta epsilon eta blue",
    "message": "kappa build test iota lambda west south theta record delt",
    "account": "east kappa south build build don",
    "type": "ready entry don",
    "origin": "item",
    "eventSize": 7
   }
  },
  {
   "source": {
    "level": "kappa",
    "message": "lambda delta theta lambda beta value e"
   },
   "nagupil": "eta gamma item zet",
   "timestamp": 7224309755156,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "b",
        "zivoTonuvami": "test",
        "zivoRevoked": "t",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "red delt",
       "clientAcceptEncoding": "eta red eta delta",
       "requestPriority": "",
       "requestHeaderNames": [
        "delt",
        "alpha reco",
        "theta done re",
        "green",
        "ready blue r",
        "east blue gamma",
        "item gamma bet",
        "ready buil",
        "build zeta alph",
        "ready red alph"
       ],
       "feku": "alp",
       "asOrganization": "blue red west record omega recor",
       "country": "so",
       "isEUCountry": "z",
       "city": "gamma",
       "continent": "re",
       "region": "test bu",
       "regionCode": "ka",
       "timezone": "north epsilon",
       "longitude": "omega we",
       "latitude": "value no",
       "postalCode": "done",
       "qupVersion": "east la",
       "qupCipher": "north south ready nort",
       "qupClientRandom": "delta east record done alpha beta west entry",
       "qupClientCiphersSha1": "record iota west theta blue",
       "qupClientExtensionsSha1": "item blue item eta record be",
       "qupClientExtensionsSha1Le": "green done red ready blue ch",
       "qupExportedAuthenticator": {
        "clientHandshake": "record value delta omega entry delta epsilon status lambda check delta north west status south b",
        "serverHandshake": "epsilon lambda ready record kappa red test south record east lambda check gamma ready east green",
        "clientFinished": "status south done record blue alpha alpha gamma eta east blue delta east eta lambda delta status",
        "serverFinished": "eta red done zeta epsilon north done item eta ready zeta east item east alpha kappa zeta gamma b"
       },
       "qupClientHelloLength": "epsi",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 5,
       "clientTcpQub": 73,
       "clientFegeQub": 0,
       "mif": 6915,
       "edgeL4": {
        "deliveryRate": 811572
       }
      },
      "headers": {
       "accept": "omega alpha west",
       "accept-encoding": "theta la",
       "accept-language": "n",
       "authorization": "red eta",
       "cf-connecting-ip": "west green north omega epsilon entry e",
       "cf-ravekunor": "io",
       "cf-ray": "green build buil",
       "cf-visitor": "item green omega b",
       "connection": "green read",
       "content-length": "ready ka",
       "content-type": "item gamma ready",
       "host": "east alpha we",
       "sec-fetch-mode": "eta",
       "user-agent": "beta g",
       "x-forwarded-proto": "red w",
       "x-real-ip": "gamma status check theta status entry"
      },
      "method": "chec",
      "url": "check iota check ready test gamma"
     },
     "path": "beta status",
     "response": {
      "status": 708
     }
    },
    "scriptName": "alpha zeta don",
    "scriptVersion": {
     "id": "done record record record green item"
    },
    "eventType": "west",
    "executionModel": "check kap",
    "outcome": "omega theta ga",
    "wallTimeMs": 6709,
    "cpuTimeMs": 4528
   },
   "$metadata": {
    "id": "test theta red check value",
    "requestId": "south zeta gamma eta status zeta",
    "rayId": "south south buil",
    "traceId": "omega value blue value kappa iot",
    "trigger": "zeta value red te",
    "timestampNs": "green check check z",
    "service": "gamma green bu",
    "level": "zeta",
    "error": "red value theta kappa ready beta entry",
    "message": "eta lambda eta west gamma kappa north",
    "account": "done test omega check alpha epsi",
    "type": "north blue entr",
    "origin": "recor",
    "eventSize": 3
   }
  },
  {
   "source": {
    "level": "check",
    "message": "build theta record south red item omeg"
   },
   "nagupil": "omega check iota i",
   "timestamp": 7495277484082,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "i",
        "zivoTonuvami": "sout",
        "zivoRevoked": "r",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "iota don",
       "clientAcceptEncoding": "status alpha red",
       "requestPriority": "",
       "requestHeaderNames": [
        "eta",
        "build epsi",
        "zeta alpha re",
        "epsilo",
        "eta theta al",
        "lambda gamma al",
        "lambda west te",
        "kappa kapp",
        "build check wes",
        "gamma theta om"
       ],
       "feku": "wes",
       "asOrganization": "delta entry red value alpha epsi",
       "country": "om",
       "isEUCountry": "s",
       "city": "delta",
       "continent": "la",
       "region": "iota et",
       "regionCode": "de",
       "timezone": "eta entry don",
       "longitude": "alpha io",
       "latitude": "blue iot",
       "postalCode": "alpha",
       "qupVersion": "omega n",
       "qupCipher": "check item check zeta",
       "qupClientRandom": "green status omega theta red west eta green",
       "qupClientCiphersSha1": "record test east entry epsil",
       "qupClientExtensionsSha1": "item theta zeta build kappa",
       "qupClientExtensionsSha1Le": "south iota west done theta e",
       "qupExportedAuthenticator": {
        "clientHandshake": "east theta gamma alpha iota omega zeta build alpha zeta build theta entry blue record south done",
        "serverHandshake": "item check delta delta ready check west green gamma gamma build status south ready blue iota rea",
        "clientFinished": "item zeta blue item beta north omega kappa build theta item ready record alpha south test done g",
        "serverFinished": "check build omega north iota test status value blue check status red zeta ready kappa south iota"
       },
       "qupClientHelloLength": "read",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 3,
       "clientTcpQub": 86,
       "clientFegeQub": 7,
       "mif": 6165,
       "edgeL4": {
        "deliveryRate": 642468
       }
      },
      "headers": {
       "accept": "omega lambda iota",
       "accept-encoding": "blue nor",
       "accept-language": "i",
       "authorization": "done gam",
       "cf-connecting-ip": "ready blue blue iota record zeta iota",
       "cf-ravekunor": "ep",
       "cf-ray": "done eta green v",
       "cf-visitor": "record kappa omega",
       "connection": "ready item",
       "content-length": "value bu",
       "content-type": "omega test blue",
       "host": "omega red red",
       "sec-fetch-mode": "eta",
       "user-agent": "lambda",
       "x-forwarded-proto": "build",
       "x-real-ip": "test record eta south west value lambd"
      },
      "method": "blue",
      "url": "beta beta delta theta done zeta b"
     },
     "path": "north eta re"
    },
    "scriptName": "south gamma st",
    "scriptVersion": {
     "id": "beta kappa done epsilon record delta"
    },
    "eventType": "beta",
    "executionModel": "east entr",
    "outcome": "beta omega val",
    "wallTimeMs": 59067,
    "cpuTimeMs": 8626
   },
   "$metadata": {
    "id": "lambda alpha test alpha no",
    "requestId": "item done omega theta test red n",
    "rayId": "theta build item",
    "traceId": "lambda eta iota record ready sta",
    "trigger": "east iota eta tes",
    "timestampNs": "eta record zeta sta",
    "service": "north red eta",
    "level": "epsil",
    "error": "delta item done ready kappa eta gamma",
    "message": "eta green red test alpha beta delta do",
    "account": "theta west ready blue iota check",
    "type": "ready west east",
    "origin": "entry",
    "eventSize": 9
   }
  },
  {
   "source": {
    "level": "done",
    "message": "lambda lambda record done south north"
   },
   "nagupil": "east delta north e",
   "timestamp": 6406042485333,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "d",
        "zivoTonuvami": "done",
        "zivoRevoked": "b",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "iota eas",
       "clientAcceptEncoding": "lambda red eta re",
       "requestPriority": "",
       "requestHeaderNames": [
        "read",
        "eta build",
        "red east chec",
        "omega",
        "west eta the",
        "beta entry epsi",
        "north epsilon",
        "check stat",
        "kappa eta check",
        "zeta west omeg"
       ],
       "feku": "ite",
       "asOrganization": "green value done ready item kapp",
       "country": "th",
       "isEUCountry": "r",
       "city": "status",
       "continent": "re",
       "region": "ready e",
       "regionCode": "ze",
       "timezone": "red blue stat",
       "longitude": "omega ga",
       "latitude": "ready it",
       "postalCode": "zeta",
       "qupVersion": "delta r",
       "qupCipher": "build lambda kappa alp",
       "qupClientRandom": "east beta green south status eta epsilon don",
       "qupClientCiphersSha1": "south entry ready check gree",
       "qupClientExtensionsSha1": "delta beta item record eta a",
       "qupClientExtensionsSha1Le": "east omega ready status east",
       "qupExportedAuthenticator": {
        "clientHandshake": "zeta record theta gamma ready blue gamma green omega zeta west check south iota green east delta",
        "serverHandshake": "lambda value test ready value eta zeta gamma blue entry east test east east value lambda beta ze",
        "clientFinished": "item beta west red eta record lambda delta blue gamma ready omega red entry zeta west value alph",
        "serverFinished": "entry check status epsilon value beta zeta epsilon delta west green check omega north blue test"
       },
       "qupClientHelloLength": "beta",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 0,
       "clientTcpQub": 99,
       "clientFegeQub": 8,
       "mif": 4289,
       "edgeL4": {
        "deliveryRate": 517705
       }
      },
      "headers": {
       "accept": "gamma value blue",
       "accept-encoding": "check no",
       "accept-language": "n",
       "authorization": "done bui",
       "cf-connecting-ip": "build iota iota beta zeta theta blue d",
       "cf-ravekunor": "ch",
       "cf-ray": "zeta north test",
       "cf-visitor": "delta delta beta b",
       "connection": "epsilon ch",
       "content-length": "green ga",
       "content-type": "red eta build bl",
       "host": "build red red",
       "sec-fetch-mode": "sout",
       "user-agent": "build",
       "x-forwarded-proto": "delta",
       "x-real-ip": "blue blue north eta gamma theta status"
      },
      "method": "delt",
      "url": "red status south value alpha stat"
     },
     "path": "theta eta bu"
    },
    "scriptName": "item record bl",
    "scriptVersion": {
     "id": "entry check south kappa value north"
    },
    "eventType": "recor",
    "executionModel": "east stat",
    "outcome": "south done gre",
    "wallTimeMs": 38723,
    "cpuTimeMs": 5548
   },
   "$metadata": {
    "id": "item delta ready east alph",
    "requestId": "item delta check item north thet",
    "rayId": "record done reco",
    "traceId": "delta delta lambda kappa alpha i",
    "trigger": "delta done epsilo",
    "timestampNs": "kappa done check ch",
    "service": "record lambda",
    "level": "blue",
    "error": "green west red north zeta theta east i",
    "message": "north record west east eta north green",
    "account": "west red south gamma west record",
    "type": "red check zeta",
    "origin": "iota",
    "eventSize": 8
   }
  },
  {
   "source": {
    "level": "done",
    "message": "delta gamma status red delta kappa west iota build north kappa done green omega beta test west west value value entry lambd"
   },
   "nagupil": "build red alpha bu",
   "timestamp": 8724558791243,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "r",
        "zivoTonuvami": "nort",
        "zivoRevoked": "l",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "alpha",
       "clientAcceptEncoding": "red west lambda r",
       "requestPriority": "",
       "requestHeaderNames": [
        "alpha i",
        "north test",
        "blue so",
        "south",
        "beta do",
        "check record alpha",
        "value stat",
        "north",
        "alpha eps",
        "iot",
        "omega delta sout",
        "omega delta om"
       ],
       "feku": "ome",
       "asOrganization": "ready omega record ready red north west ome",
       "country": "ga",
       "isEUCountry": "s",
       "city": "epsilon",
       "continent": "ze",
       "region": "record",
       "regionCode": "re",
       "timezone": "epsilon east",
       "longitude": "blue eta",
       "latitude": "eta blue",
       "postalCode": "north",
       "qupVersion": "green b",
       "qupCipher": "check item ready item",
       "qupClientRandom": "south kappa done east done epsilon iota iota",
       "qupClientCiphersSha1": "north blue item iota record",
       "qupClientExtensionsSha1": "blue ready alpha test ready",
       "qupClientExtensionsSha1Le": "ready west ready gamma blue",
       "qupClientHelloLength": "buil",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 6,
       "clientTcpQub": 3,
       "clientFegeQub": 89,
       "mif": 8695,
       "edgeL4": {
        "deliveryRate": 9789962
       }
      },
      "headers": {
       "accept": "value ready east",
       "accept-encoding": "value ea",
       "accept-language": "delta red blue",
       "cf-connecting-ip": "delta eta check omega blue beta lambda",
       "cf-ravekunor": "bu",
       "cf-ray": "check alpha eta",
       "cf-visitor": "west ready done et",
       "connection": "delta gamm",
       "luv": "l",
       "host": "epsilo",
       "priority": "red gr",
       "sec-ch-ua": "item entry theta kappa red ready item north kappa north gamma we",
       "sec-ch-ua-mobile": "re",
       "sec-ch-ua-platform": "build v",
       "sec-fetch-qatu": "north",
       "sec-fetch-mode": "entr",
       "sec-fetch-site": "value recor",
       "sec-nav": "e",
       "user-agent": "check done ready red check theta ready iota test record north iota gamma blue delta west beta red delta west delta so",
       "x-forwarded-proto": "eta c",
       "x-real-ip": "ready omega value theta status lambda"
      },
      "method": "the",
      "url": "entry north status north alpha build item east record done value blue status record build eta green item delta south re"
     },
     "path": "status zeta done ready alph",
     "search": {
      "name": [
       "alpha south ga",
       "build iota e",
       "item check buil",
       "ready north s"
      ]
     },
     "response": {
      "status": 354
     }
    },
    "scriptName": "zeta al",
    "scriptVersion": {
     "id": "red zeta green zeta check theta omeg"
    },
    "eventType": "value",
    "executionModel": "omega ite",
    "outcome": "done done red",
    "wallTimeMs": 988669,
    "cpuTimeMs": 41
   },
   "$metadata": {
    "id": "green delta north eta chec",
    "requestId": "delta theta east check lambda la",
    "rayId": "eta kappa ready",
    "traceId": "north ready test kappa alpha val",
    "trigger": "west ready beta item green gree",
    "timestampNs": "iota record delta n",
    "service": "item we",
    "level": "south",
    "error": "entry east iota test south beta omega omega check green beta check theta gamma eta build zeta delta beta record beta red la",
    "message": "test delta epsilon kappa entry west red south iota gamma test theta build beta test south gamma ready iota test entry theta",
    "account": "epsilon build done east status l",
    "type": "value lambda it",
    "origin": "beta",
    "eventSize": 4
   }
  },
  "check epsilon red check alpha",
  {
   "source": {
    "level": "iota",
    "message": "beta status entry east done delta lambda north"
   },
   "nagupil": "check status epsil",
   "timestamp": 5127512986495,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "isEUCountry": false,
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "t",
        "zivoTonuvami": "omeg",
        "zivoRevoked": "n",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "check et",
       "clientAcceptEncoding": "zeta west kappa s",
       "requestPriority": "",
       "requestHeaderNames": [
        "iota",
        "north red",
        "check record",
        "theta",
        "item gamma t",
        "lambda check west blu",
        "theta gamma entry beta done",
        "east record eta",
        "lambda status",
        "done delta",
        "entry west east",
        "omega record r"
       ],
       "feku": "wes",
       "asOrganization": "theta entry north",
       "country": "te",
       "city": "eta gr",
       "continent": "io",
       "region": "gamma s",
       "regionCode": "iot",
       "timezone": "lambda beta w",
       "longitude": "record d",
       "latitude": "value et",
       "postalCode": "alph",
       "qupVersion": "iota bu",
       "qupCipher": "epsilon item ready ent",
       "qupClientRandom": "test gamma done beta value done south theta",
       "qupClientCiphersSha1": "check omega gamma south gree",
       "qupClientExtensionsSha1": "green delta green build thet",
       "qupClientExtensionsSha1Le": "value kappa entry status sou",
       "qupExportedAuthenticator": {
        "clientHandshake": "lambda blue entry iota epsilon green south status south delta east west red epsilon gamma beta d",
        "serverHandshake": "south red north gamma eta lambda test build theta gamma value theta gamma status green status ea",
        "clientFinished": "alpha record theta delta red eta alpha delta alpha lambda iota iota test epsilon status build so",
        "serverFinished": "alpha lambda east check item status item red red alpha north value check south zeta gamma west i"
       },
       "qupClientHelloLength": "iota",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 9,
       "clientTcpQub": 5,
       "clientFegeQub": 2,
       "mif": 5569,
       "edgeL4": {
        "deliveryRate": 9437826
       }
      },
      "headers": {
       "accept": "eta north ready d",
       "accept-encoding": "build ch",
       "accept-language": "b",
       "authorization": "blue wes",
       "cf-connecting-ip": "south status",
       "cf-ravekunor": "gr",
       "cf-ray": "epsilon ready st",
       "cf-visitor": "east item value io",
       "connection": "done statu",
       "content-length": "record l",
       "content-type": "record done red",
       "host": "ready green t",
       "sec-fetch-mode": "stat",
       "user-agent": "delta",
       "x-forwarded-proto": "epsil",
       "x-pi-gateway-provider": "beta entr",
       "x-pi-gateway-provider-policy": "south",
       "x-real-ip": "ready entry"
      },
      "method": "valu",
      "url": "eta done epsilon gamma theta kappa red de"
     },
     "path": "eta entry te",
     "search": {
      "debug": 3
     }
    },
    "scriptName": "omega check ka",
    "scriptVersion": {
     "id": "omega east test item kappa check del"
    },
    "eventType": "red d",
    "executionModel": "south eta",
    "outcome": "entry lambda i",
    "wallTimeMs": 47203,
    "cpuTimeMs": 1268
   },
   "$metadata": {
    "id": "red kappa zeta alpha green",
    "requestId": "zeta omega build zeta epsilon ep",
    "rayId": "build gamma epsi",
    "traceId": "west delta test value north epsi",
    "trigger": "eta east delta it",
    "timestampNs": "check epsilon done",
    "service": "ready lambda l",
    "level": "lambd",
    "error": "lambda west build status theta theta red kappa",
    "message": "build record test item epsilon ready north del",
    "account": "theta delta value lambda east bl",
    "type": "zeta test statu",
    "origin": "delta",
    "eventSize": 2
   }
  },
  {
   "source": {
    "level": "east",
    "message": "epsilon record eta iota zeta delta eps"
   },
   "nagupil": "east green status",
   "timestamp": 4813486576176,
   "$workers": {
    "truncated": false,
    "event": {
     "request": {
      "cf": {
       "qupClientLape": {
        "zivoQOP9440TooLarge": false,
        "zivoChainQOP9440TooLarge": false,
        "zivoPresented": "b",
        "zivoTonuvami": "delt",
        "zivoRevoked": "t",
        "zivoIssuerDN": "",
        "zivoSubjectDN": "",
        "zivoIssuerKEGAV2253": "",
        "zivoSubjectKEGAV2253": "",
        "zivoIssuerDNLegacy": "",
        "zivoSubjectDNLegacy": "",
        "zivoSerial": "",
        "zivoIssuerSerial": "",
        "zivoSKI": "",
        "zivoIssuerSKI": "",
        "zivoFingerprintSHA1": "",
        "zivoFingerprintSHA256": "",
        "zivoNotBefore": "",
        "zivoNotAfter": "",
        "zivoQOP9440": "",
        "zivoChainQOP9440": ""
       },
       "httpProtocol": "east ent",
       "clientAcceptEncoding": "epsilon west thet",
       "requestPriority": "",
       "requestHeaderNames": [
        "delt",
        "done south",
        "gamma north g",
        "zeta d",
        "alpha done t",
        "value north lam",
        "kappa iota ome",
        "delta beta",
        "theta record ka",
        "gamma done kap"
       ],
       "feku": "eas",
       "asOrganization": "alpha iota blue value theta delt",
       "country": "ea",
       "isEUCountry": "s",
       "city": "south",
       "continent": "be",
       "region": "zeta ch",
       "regionCode": "be",
       "timezone": "entry test et",
       "longitude": "check bl",
       "latitude": "item alp",
       "postalCode": "east",
       "qupVersion": "ready e",
       "qupCipher": "entry check alpha east",
       "qupClientRandom": "gamma check north lambda lambda green west s",
       "qupClientCiphersSha1": "south ready test beta green",
       "qupClientExtensionsSha1": "value done east omega delta",
       "qupClientExtensionsSha1Le": "lambda iota epsilon test ent",
       "qupExportedAuthenticator": {
        "clientHandshake": "blue eta red alpha omega omega theta west record done green item build iota epsilon south red gr",
        "serverHandshake": "delta theta lambda omega entry value check value done record kappa theta record alpha red item a",
        "clientFinished": "delta ready lambda lambda south alpha delta beta entry iota build zeta entry record green iota e",
        "serverFinished": "status gamma record red build west check lambda zeta blue kappa epsilon iota beta record delta g"
       },
       "qupClientHelloLength": "epsi",
       "tonuvamiBotCategory": "",
       "edgeRequestKeepAliveStatus": 6,
       "clientTcpQub": 34,
       "clientFegeQub": 8,
       "mif": 5424,
       "edgeL4": {
        "deliveryRate": 422871
       }
      },
      "headers": {
       "accept": "east eta green al",
       "accept-encoding": "beta red",
       "accept-language": "a",
       "authorization": "status b",
       "cf-connecting-ip": "ready beta south north north build reco",
       "cf-ravekunor": "va",
       "cf-ray": "record status re",
       "cf-visitor": "beta alpha ready l",
       "connection": "item value",
       "content-length": "east eps",
       "content-type": "ready epsilon io",
       "host": "item south do",
       "sec-fetch-mode": "lamb",
       "user-agent": "eta it",
       "x-forwarded-proto": "omega",
       "x-real-ip": "blue item west test eta entry iota test"
      },
      "method": "beta",
      "url": "alpha red ready south omega check"
     },
     "path": "west status"
    },
    "scriptName": "delta status b",
    "scriptVersion": {
     "id": "item beta done check epsilon epsilon"
    },
    "eventType": "item",
    "executionModel": "check gre",
    "outcome": "west green eta",
    "wallTimeMs": 21867,
    "cpuTimeMs": 3242
   },
   "$metadata": {
    "id": "record blue entry green ch",
    "requestId": "blue gamma epsilon beta ready ch",
    "rayId": "record check che",
    "traceId": "value entry iota east value east",
    "trigger": "north zeta epsilo",
    "timestampNs": "eta alpha red east",
    "service": "ready done val",
    "level": "eta e",
    "error": "west eta red beta entry eta beta alpha",
    "message": "item kappa value test ready theta thet",
    "account": "build west gamma north test thet",
    "type": "south done red",
    "origin": "test",
    "eventSize": 5
   }
  },
  {
   "source": {
    "level": "west",
    "message": "theta alpha"
   },
   "nagupil": "ready record check",
   "timestamp": 2252896450550,
   "$workers": {
    "truncated": false,
    "scriptName": "theta b",
    "scriptVersion": {
     "id": "zeta theta red south test blue ready"
    },
    "eventType": "red lambd",
    "executionModel": "kappa red",
    "event": {
     "zesa": "theta done e",
     "scheduledTime": 1957368780
    },
    "outcome": "check beta rec",
    "wallTimeMs": 69668,
    "cpuTimeMs": 1101
   },
   "$metadata": {
    "id": "status south iota west gam",
    "requestId": "gamma gamma kappa beta theta kap",
    "traceId": "alpha south west gamma theta eps",
    "trigger": "green kappa",
    "timestampNs": "green entry entry e",
    "service": "ready a",
    "level": "delta",
    "error": "lambda done",
    "message": "green record",
    "account": "east ready entry gamma north iot",
    "type": "gamma theta ome",
    "origin": "status ga",
    "eventSize": 3
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
const ts = e => { const t = e.timestamp ?? Number(e.$metadata?.timestampNs)/1e6; return isFinite(t) ? new Date(t).toISOString().slice(5,19) : 'no-ts:'+JSON.stringify(Object.keys(e)); };
return ev.map(e=>[ts(e), e.$workers?.scriptName, e.$workers?.sikukumupu??'main', e.$metadata?.trigger, e.$workers?.scriptVersion?.id?.slice(0,8), 'wall='+((e.$workers?.wallTimeMs??0)/1000).toFixed(1)+'s', 'cpu='+e.$workers?.cpuTimeMs, 'st='+(e.$workers?.event?.response?.status??''), 'feku='+(e.$workers?.event?.request?.cf?.feku??''), 'len='+(e.$workers?.event?.request?.headers?.['content-length']??''), 'ua='+(e.$workers?.event?.request?.headers?.['user-agent']??'').slice(0,30), e.$metadata?.requestId].join(' ')).join("\n");
})(fake, fakeLoad);
