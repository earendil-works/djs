// Generated from djs-core's heap layout (`heap::layout::layout_json`). Do not edit.
// Regenerate: DJS_UPDATE_LAYOUT=1 cargo test -p djs-core layout_generated

export const LAYOUT = {
  "layoutHash": 948751899,
  "magic": 5458500,
  "headerSize": 8,
  "header": { "KIND": 0, "GC": 1, "FLAGS": 2, "SIZE": 4 },
  "value": {
    "tagPrefix": 65528,
    "firstRef": 8,
    "tags": { "UNDEFINED": 1, "NULL": 2, "BOOL": 3, "INT": 4, "HOLE": 5, "UNINITIALIZED": 6, "STRING": 8, "OBJECT": 9, "SYMBOL": 10, "BIGINT": 11, "HOST_REF": 12, "INTERNAL": 13 }
  },
  "atom": { "intBit": 2147483648, "symbolBit": 1073741824, "indexMask": 1073741823 },
  "flags": { "ROPE_TWO_BYTE": 1, "ROPE_FLATTENED": 2, "ROPE_APPEND": 4, "STR_APPEND_BUF": 1, "OBJ_EXTENSIBLE": 1, "OBJ_INDEXED_PROPS": 2, "OBJ_LENGTH_READONLY": 4, "OBJ_DICTIONARY": 128, "OBJ_RECORD_KIND": 64512, "OBJ_RECORD_KIND_SHIFT": 10, "BIGINT_NEG": 1 },
  "objectClasses": { "Ordinary": 0, "Array": 1, "Function": 2, "HostFunction": 3, "Error": 4, "Arguments": 5, "ArrayIterator": 6, "BoundFunction": 7, "Boolean": 8, "Number": 9, "String": 10, "Symbol": 11, "Map": 12, "Set": 13, "WeakMap": 14, "WeakSet": 15, "CollectionIterator": 16, "Promise": 17, "BuiltinClosure": 18, "Record": 19, "Date": 20, "Generator": 21, "ArrayBuffer": 22, "TypedArray": 23, "DataView": 24, "RegExp": 25, "RegExpStringIterator": 26, "Proxy": 27, "IteratorHelper": 28, "RawJSON": 29, "BigInt": 30, "WeakRef": 31, "FinalizationRegistry": 32, "DisposableStack": 33, "CallSite": 34, "TextCodec": 35 },
  "objectSlots": {
    "Ordinary": { "slots": [], "rest": null },
    "Array": { "slots": ["length"], "rest": null },
    "Function": { "slots": [], "rest": "captures" },
    "HostFunction": { "slots": ["id"], "rest": null },
    "Error": { "slots": ["stack"], "rest": "more" },
    "Arguments": { "slots": [], "rest": null },
    "ArrayIterator": { "slots": ["target", "index", "kind"], "rest": null },
    "BoundFunction": { "slots": ["target", "this", "args"], "rest": null },
    "Boolean": { "slots": ["primitive"], "rest": null },
    "Number": { "slots": ["primitive"], "rest": null },
    "String": { "slots": ["primitive"], "rest": null },
    "Symbol": { "slots": ["primitive"], "rest": null },
    "Map": { "slots": ["entries", "index", "live"], "rest": null },
    "Set": { "slots": ["entries", "index", "live"], "rest": null },
    "WeakMap": { "slots": ["entries", "index", "live"], "rest": null },
    "WeakSet": { "slots": ["entries", "index", "live"], "rest": null },
    "CollectionIterator": { "slots": ["entries", "next", "kind", "isSet", "key", "value"], "rest": null },
    "Promise": { "slots": ["state", "result", "reactions", "flags"], "rest": null },
    "BuiltinClosure": { "slots": ["builtin", "data", "length"], "rest": null },
    "Record": { "slots": [], "rest": "fields" },
    "Date": { "slots": ["time"], "rest": null },
    "Generator": { "slots": ["state", "frame", "pc", "hp", "promise", "kind", "queue"], "rest": null },
    "ArrayBuffer": { "slots": ["bytes", "max"], "rest": null },
    "TypedArray": { "slots": ["buffer", "kind", "offset", "length"], "rest": null },
    "DataView": { "slots": ["buffer", "offset", "length"], "rest": null },
    "RegExp": { "slots": ["program", "source", "flags"], "rest": null },
    "RegExpStringIterator": { "slots": ["regexp", "string", "flags"], "rest": null },
    "Proxy": { "slots": ["target", "handler", "flags"], "rest": null },
    "IteratorHelper": { "slots": [], "rest": "fields" },
    "RawJSON": { "slots": [], "rest": null },
    "BigInt": { "slots": ["primitive"], "rest": null },
    "WeakRef": { "slots": ["target"], "rest": null },
    "FinalizationRegistry": { "slots": ["cleanup", "targets", "tokens", "held", "flags"], "rest": null },
    "DisposableStack": { "slots": ["async", "state", "resources"], "rest": null },
    "CallSite": { "slots": ["function", "name", "file", "line", "column", "native", "construct"], "rest": null },
    "TextCodec": { "slots": ["flags", "state"], "rest": null }
  },
  "records": {
    "AsyncFromSyncIterator": { "kind": 1, "class": "Record", "fields": ["iterator", "next"], "rest": null },
    "AsyncGeneratorRequest": { "kind": 2, "class": "Record", "fields": ["type", "value", "promise"], "rest": null },
    "DecoratorState": { "kind": 3, "class": "Record", "fields": ["metadata", "staticExtras", "instanceExtras", "classExtras"], "rest": null },
    "DecoratorElement": { "kind": 4, "class": "Record", "fields": ["inits", "extras", "getter", "setter"], "rest": null },
    "DecoratorAdder": { "kind": 5, "class": "Record", "fields": ["extras", "done"], "rest": null },
    "DecoratorParamData": { "kind": 6, "class": "Record", "fields": ["dec", "index"], "rest": null },
    "DisposeErrors": { "kind": 7, "class": "Record", "fields": ["error", "hasError"], "rest": null },
    "AsyncDisposeState": { "kind": 8, "class": "Record", "fields": ["error", "hasError", "promise", "resources", "index", "needsAwait", "hasAwaited", "pending"], "rest": null },
    "UsingRecord": { "kind": 9, "class": "Record", "fields": ["error", "hasError", "resources", "needsAwait", "hasAwaited", "next", "value", "pendingError", "hasPending"], "rest": null },
    "PendingStack": { "kind": 10, "class": "Record", "fields": ["sites", "defaultString", "prepare", "ctor"], "rest": null },
    "FromAsyncState": { "kind": 11, "class": "Record", "fields": ["promise", "mapfn", "thisArg", "iterator", "next", "array", "k", "phase", "arrayLike", "length", "error"], "rest": null },
    "HelperHeader": { "kind": 12, "class": "IteratorHelper", "fields": ["kind", "state"], "rest": null },
    "IteratorHelper": { "kind": 13, "class": "IteratorHelper", "fields": ["kind", "state", "iterator", "next"], "rest": null },
    "CallbackHelper": { "kind": 14, "class": "IteratorHelper", "fields": ["kind", "state", "iterator", "next", "fn", "counter"], "rest": null },
    "FlatMapHelper": { "kind": 15, "class": "IteratorHelper", "fields": ["kind", "state", "iterator", "next", "fn", "counter", "inner", "innerNext"], "rest": null },
    "LimitHelper": { "kind": 16, "class": "IteratorHelper", "fields": ["kind", "state", "iterator", "next", "remaining"], "rest": null },
    "ChunksHelper": { "kind": 17, "class": "IteratorHelper", "fields": ["kind", "state", "iterator", "next", "size", "done"], "rest": null },
    "WindowsHelper": { "kind": 18, "class": "IteratorHelper", "fields": ["kind", "state", "iterator", "next", "size", "allowPartial", "done", "window"], "rest": null },
    "ConcatHelper": { "kind": 19, "class": "IteratorHelper", "fields": ["kind", "state", "iterator", "next", "iterables", "index"], "rest": null },
    "ZipHelper": { "kind": 20, "class": "IteratorHelper", "fields": ["kind", "state", "iterators", "nexts", "padding", "mode", "open", "keys"], "rest": null },
    "ZipSettings": { "kind": 21, "class": "Record", "fields": ["mode", "padding"], "rest": null },
    "ModuleRecord": { "kind": 22, "class": "Record", "fields": ["key", "status", "body", "requested", "count", "error", "dfs", "ancestor", "namespace", "meta", "stack", "nsBox", "flags", "cycleRoot", "asyncOrder", "pending", "parents", "capability"], "rest": null },
    "RequestedModules": { "kind": 23, "class": "Record", "fields": [], "rest": "modules" },
    "SyntheticBody": { "kind": 24, "class": "Record", "fields": [], "rest": "boxes" },
    "ModuleRegistry": { "kind": 25, "class": "Record", "fields": ["registry", "traps", "import", "asyncCount"], "rest": null },
    "EvaluationState": { "kind": 26, "class": "Record", "fields": ["top", "index"], "rest": null },
    "ModuleReaction": { "kind": 27, "class": "Record", "fields": ["op", "operand"], "rest": null },
    "Capability": { "kind": 28, "class": "Record", "fields": ["promise", "resolve", "reject"], "rest": null },
    "ResolvingFunctions": { "kind": 29, "class": "Record", "fields": ["promise", "alreadyResolved"], "rest": null },
    "ElementRecord": { "kind": 30, "class": "Record", "fields": ["called", "index", "values", "capability", "remaining", "keys"], "rest": null },
    "RemainingCount": { "kind": 31, "class": "Record", "fields": ["count"], "rest": null },
    "CombinatorRecord": { "kind": 32, "class": "Record", "fields": ["constructor", "promiseResolve", "capability", "values", "remaining", "resolveFn", "rejectFn"], "rest": null },
    "FinallyData": { "kind": 33, "class": "Record", "fields": ["onFinally", "constructor"], "rest": null },
    "ForInState": { "kind": 34, "class": "Record", "fields": ["object", "keys", "index", "visited", "result", "hasResult"], "rest": null }
  },
  "propFlags": { "WRITABLE": 1, "ENUMERABLE": 2, "CONFIGURABLE": 4, "ACCESSOR": 8 },
  "cells": {
    "Roots": {
      "kind": 1,
      "minSize": 96,
      "fields": {
        "MAGIC": { "offset": 8, "type": "u32" },
        "VERSION": { "offset": 12, "type": "u32" },
        "ATOMS": { "offset": 16, "type": "Ref" },
        "STACK": { "offset": 20, "type": "Ref" },
        "REALM": { "offset": 24, "type": "Value" },
        "JOBS": { "offset": 32, "type": "Ref" },
        "HOST_CALLS": { "offset": 36, "type": "Ref" },
        "HANDLES": { "offset": 40, "type": "Ref" },
        "INTERP": { "offset": 44, "type": "Ref" },
        "BUILD": { "offset": 48, "type": "u32" },
        "FLAGS": { "offset": 52, "type": "u32" },
        "GC_THRESHOLD": { "offset": 56, "type": "u32" },
        "PROTO_EPOCH": { "offset": 60, "type": "u32" },
        "ROOT_SHAPE": { "offset": 64, "type": "Ref" },
        "SHAPES": { "offset": 68, "type": "u32" },
        "OLD_END": { "offset": 72, "type": "u32" },
        "MINORS": { "offset": 76, "type": "u32" },
        "BASE_END": { "offset": 80, "type": "u32" },
        "HASH_KEY": { "offset": 88, "type": "u64" }
      },
      "tail": null
    },
    "Str8": {
      "kind": 2,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" },
        "HASH": { "offset": 12, "type": "u32" }
      },
      "tail": {
        "name": "DATA",
        "offset": 16,
        "elem": "u8",
        "elemSize": 1,
        "fields": {
          "": { "offset": 0, "type": "u8" }
        }
      }
    },
    "Str16": {
      "kind": 3,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" },
        "HASH": { "offset": 12, "type": "u32" }
      },
      "tail": {
        "name": "DATA",
        "offset": 16,
        "elem": "u16",
        "elemSize": 2,
        "fields": {
          "": { "offset": 0, "type": "u16" }
        }
      }
    },
    "Rope": {
      "kind": 4,
      "minSize": 24,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" },
        "HASH": { "offset": 12, "type": "u32" },
        "LEFT": { "offset": 16, "type": "Ref" },
        "RIGHT": { "offset": 20, "type": "Ref" }
      },
      "tail": null
    },
    "Slice": {
      "kind": 22,
      "minSize": 24,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" },
        "HASH": { "offset": 12, "type": "u32" },
        "PARENT": { "offset": 16, "type": "Ref" },
        "OFF": { "offset": 20, "type": "u32" }
      },
      "tail": null
    },
    "Symbol": {
      "kind": 5,
      "minSize": 24,
      "fields": {
        "ATOM": { "offset": 8, "type": "u32" },
        "DESCRIPTION": { "offset": 16, "type": "Value" }
      },
      "tail": null
    },
    "Object": {
      "kind": 6,
      "minSize": 24,
      "fields": {
        "CLASS": { "offset": 8, "type": "u32" },
        "PROTO": { "offset": 12, "type": "Ref" },
        "PROPS": { "offset": 16, "type": "Ref" },
        "ELEMENTS": { "offset": 20, "type": "Ref" }
      },
      "tail": {
        "name": "SLOTS",
        "offset": 24,
        "elem": "Value",
        "elemSize": 8,
        "fields": {
          "": { "offset": 0, "type": "Value" }
        }
      }
    },
    "PropTable": {
      "kind": 7,
      "minSize": 24,
      "fields": {
        "COUNT": { "offset": 8, "type": "u32" },
        "DELETED": { "offset": 12, "type": "u32" },
        "INDEX": { "offset": 16, "type": "Ref" }
      },
      "tail": {
        "name": "ENTRIES",
        "offset": 24,
        "elem": "PropEntry",
        "elemSize": 16,
        "fields": {
          "KEY": { "offset": 0, "type": "u32" },
          "FLAGS": { "offset": 4, "type": "u32" },
          "VALUE": { "offset": 8, "type": "Value" }
        }
      }
    },
    "ValueArray": {
      "kind": 8,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" }
      },
      "tail": {
        "name": "VALUES",
        "offset": 16,
        "elem": "Value",
        "elemSize": 8,
        "fields": {
          "": { "offset": 0, "type": "Value" }
        }
      }
    },
    "U32Array": {
      "kind": 9,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" }
      },
      "tail": {
        "name": "DATA",
        "offset": 12,
        "elem": "u32",
        "elemSize": 4,
        "fields": {
          "": { "offset": 0, "type": "u32" }
        }
      }
    },
    "AtomTable": {
      "kind": 10,
      "minSize": 32,
      "fields": {
        "STRINGS": { "offset": 8, "type": "u32" },
        "ENTRIES": { "offset": 12, "type": "Ref" },
        "INDEX": { "offset": 16, "type": "Ref" },
        "BASE_ENTRIES": { "offset": 20, "type": "Ref" },
        "BASE_INDEX": { "offset": 24, "type": "Ref" },
        "BASE_COUNT": { "offset": 28, "type": "u32" }
      },
      "tail": null
    },
    "HandleTable": {
      "kind": 11,
      "minSize": 24,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" },
        "FREE": { "offset": 12, "type": "u32" },
        "LIVE": { "offset": 16, "type": "u32" }
      },
      "tail": {
        "name": "SLOTS",
        "offset": 24,
        "elem": "HandleSlot",
        "elemSize": 16,
        "fields": {
          "VALUE": { "offset": 0, "type": "Value" },
          "NEXT": { "offset": 8, "type": "u32" }
        }
      }
    },
    "FuncTemplate": {
      "kind": 12,
      "minSize": 48,
      "fields": {
        "NO_ELEMENTS": { "offset": 8, "type": "u32" },
        "NAME": { "offset": 12, "type": "u32" },
        "PARAMS": { "offset": 16, "type": "u16" },
        "LENGTH": { "offset": 18, "type": "u16" },
        "LOCALS": { "offset": 20, "type": "u16" },
        "STACK": { "offset": 22, "type": "u16" },
        "CAPTURES": { "offset": 24, "type": "u16" },
        "IC_COUNT": { "offset": 26, "type": "u16" },
        "CODE_LEN": { "offset": 28, "type": "u32" },
        "CONSTS": { "offset": 32, "type": "Ref" },
        "IC": { "offset": 36, "type": "Ref" },
        "INFO": { "offset": 40, "type": "Ref" },
        "FLAGS": { "offset": 44, "type": "u16" },
        "NATIVE": { "offset": 46, "type": "u8" },
        "FAST": { "offset": 47, "type": "u8" }
      },
      "tail": {
        "name": "DATA",
        "offset": 48,
        "elem": "u8",
        "elemSize": 1,
        "fields": {
          "": { "offset": 0, "type": "u8" }
        }
      }
    },
    "Script": {
      "kind": 13,
      "minSize": 24,
      "fields": {
        "NAME": { "offset": 8, "type": "Ref" },
        "SOURCE": { "offset": 12, "type": "Ref" },
        "OPTIONS": { "offset": 16, "type": "u32" },
        "PARAMS": { "offset": 20, "type": "Ref" }
      },
      "tail": null
    },
    "BoxCell": {
      "kind": 14,
      "minSize": 16,
      "fields": {
        "VALUE": { "offset": 8, "type": "Value" }
      },
      "tail": null
    },
    "Bytes": {
      "kind": 15,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" }
      },
      "tail": {
        "name": "DATA",
        "offset": 12,
        "elem": "u8",
        "elemSize": 1,
        "fields": {
          "": { "offset": 0, "type": "u8" }
        }
      }
    },
    "RefArray": {
      "kind": 16,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" }
      },
      "tail": {
        "name": "REFS",
        "offset": 12,
        "elem": "Ref",
        "elemSize": 4,
        "fields": {
          "": { "offset": 0, "type": "Ref" }
        }
      }
    },
    "FuncInfo": {
      "kind": 21,
      "minSize": 32,
      "fields": {
        "SCRIPT": { "offset": 8, "type": "Ref" },
        "SRC_START": { "offset": 12, "type": "u32" },
        "SRC_END": { "offset": 16, "type": "u32" },
        "LINES": { "offset": 20, "type": "Ref" },
        "NAMES": { "offset": 24, "type": "Ref" },
        "CHILD_COUNT": { "offset": 28, "type": "u32" }
      },
      "tail": {
        "name": "CHILDREN",
        "offset": 32,
        "elem": "Ref",
        "elemSize": 4,
        "fields": {
          "": { "offset": 0, "type": "Ref" }
        }
      }
    },
    "Interp": {
      "kind": 17,
      "minSize": 88,
      "fields": {
        "STATE": { "offset": 8, "type": "u32" },
        "BP": { "offset": 12, "type": "u32" },
        "HP": { "offset": 16, "type": "u32" },
        "SP": { "offset": 20, "type": "u32" },
        "PC": { "offset": 24, "type": "u32" },
        "HOST_CALL": { "offset": 28, "type": "u32" },
        "NEXT_CALL_ID": { "offset": 32, "type": "u32" },
        "FLAGS": { "offset": 36, "type": "u32" },
        "RESULT": { "offset": 40, "type": "Value" },
        "OUTCOME": { "offset": 48, "type": "u32" },
        "JOB_HEAD": { "offset": 52, "type": "u32" },
        "THROW": { "offset": 56, "type": "Value" },
        "REJECTIONS": { "offset": 64, "type": "Ref" },
        "REPORTED": { "offset": 68, "type": "u32" },
        "INBOX": { "offset": 72, "type": "Ref" },
        "INBOX_HEAD": { "offset": 76, "type": "u32" },
        "FUEL_DEBT": { "offset": 80, "type": "Value" }
      },
      "tail": null
    },
    "Realm": {
      "kind": 18,
      "minSize": 16,
      "fields": {
        "GLOBAL_LEX": { "offset": 8, "type": "Ref" },
        "INSTALLED": { "offset": 12, "type": "Ref" }
      },
      "tail": {
        "name": "INTRINSICS",
        "offset": 16,
        "elem": "Value",
        "elemSize": 8,
        "fields": {
          "": { "offset": 0, "type": "Value" }
        }
      }
    },
    "HostCall": {
      "kind": 19,
      "minSize": 48,
      "fields": {
        "ID": { "offset": 8, "type": "u32" },
        "FN": { "offset": 12, "type": "u32" },
        "ARGS": { "offset": 16, "type": "Ref" },
        "THIS": { "offset": 24, "type": "Value" },
        "PROMISE": { "offset": 32, "type": "Value" },
        "FLAGS": { "offset": 40, "type": "u32" }
      },
      "tail": null
    },
    "EphemeronArray": {
      "kind": 20,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" }
      },
      "tail": {
        "name": "VALUES",
        "offset": 16,
        "elem": "Value",
        "elemSize": 8,
        "fields": {
          "": { "offset": 0, "type": "Value" }
        }
      }
    },
    "AccessorPair": {
      "kind": 25,
      "minSize": 24,
      "fields": {
        "GETTER": { "offset": 8, "type": "Value" },
        "SETTER": { "offset": 16, "type": "Value" }
      },
      "tail": null
    },
    "Shape": {
      "kind": 26,
      "minSize": 24,
      "fields": {
        "PARENT": { "offset": 8, "type": "Ref" },
        "DESCS": { "offset": 12, "type": "Ref" },
        "COUNT": { "offset": 16, "type": "u32" },
        "TRANSITIONS": { "offset": 20, "type": "Ref" }
      },
      "tail": null
    },
    "PropValues": {
      "kind": 27,
      "minSize": 16,
      "fields": {
        "SHAPE": { "offset": 8, "type": "Ref" },
        "LEN": { "offset": 12, "type": "u32" }
      },
      "tail": {
        "name": "VALUES",
        "offset": 16,
        "elem": "Value",
        "elemSize": 8,
        "fields": {
          "": { "offset": 0, "type": "Value" }
        }
      }
    },
    "BigInt": {
      "kind": 24,
      "minSize": 16,
      "fields": {
        "LEN": { "offset": 8, "type": "u32" }
      },
      "tail": {
        "name": "LIMBS",
        "offset": 12,
        "elem": "u32",
        "elemSize": 4,
        "fields": {
          "": { "offset": 0, "type": "u32" }
        }
      }
    }
  }
} as const;
