// Generated from djs-core's heap layout (`heap::layout::layout_json`). Do not edit.
// Regenerate: DJS_UPDATE_LAYOUT=1 cargo test -p djs-core layout_generated

export const LAYOUT = {
  "layoutHash": 2168310655,
  "magic": 5458500,
  "headerSize": 8,
  "header": { "KIND": 0, "GC": 1, "FLAGS": 2, "SIZE": 4 },
  "value": {
    "tagPrefix": 65528,
    "firstRef": 8,
    "tags": { "UNDEFINED": 1, "NULL": 2, "BOOL": 3, "INT": 4, "HOLE": 5, "UNINITIALIZED": 6, "STRING": 8, "OBJECT": 9, "SYMBOL": 10, "BIGINT": 11, "HOST_REF": 12, "INTERNAL": 13 }
  },
  "atom": { "intBit": 2147483648, "symbolBit": 1073741824, "indexMask": 1073741823 },
  "flags": { "ROPE_TWO_BYTE": 1, "ROPE_FLATTENED": 2, "OBJ_EXTENSIBLE": 1, "OBJ_INDEXED_PROPS": 2, "OBJ_LENGTH_READONLY": 4 },
  "objectClasses": { "Ordinary": 0, "Array": 1, "Function": 2, "HostFunction": 3, "Error": 4, "Arguments": 5 },
  "propFlags": { "WRITABLE": 1, "ENUMERABLE": 2, "CONFIGURABLE": 4, "ACCESSOR": 8 },
  "cells": {
    "Roots": {
      "kind": 1,
      "minSize": 48,
      "fields": {
        "MAGIC": { "offset": 8, "type": "u32" },
        "VERSION": { "offset": 12, "type": "u32" },
        "ATOMS": { "offset": 16, "type": "Ref" },
        "STACK": { "offset": 20, "type": "Ref" },
        "REALM": { "offset": 24, "type": "Value" },
        "JOBS": { "offset": 32, "type": "Ref" },
        "HOST_CALLS": { "offset": 36, "type": "Ref" },
        "HANDLES": { "offset": 40, "type": "Ref" },
        "INTERP": { "offset": 44, "type": "Ref" }
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
        "elemSize": 24,
        "fields": {
          "KEY": { "offset": 0, "type": "u32" },
          "FLAGS": { "offset": 4, "type": "u32" },
          "VALUE": { "offset": 8, "type": "Value" },
          "SETTER": { "offset": 16, "type": "Value" }
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
      "minSize": 24,
      "fields": {
        "STRINGS": { "offset": 8, "type": "u32" },
        "ENTRIES": { "offset": 12, "type": "Ref" },
        "INDEX": { "offset": 16, "type": "Ref" }
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
      "minSize": 72,
      "fields": {
        "FLAGS": { "offset": 8, "type": "u32" },
        "NAME": { "offset": 12, "type": "u32" },
        "PARAMS": { "offset": 16, "type": "u16" },
        "LENGTH": { "offset": 18, "type": "u16" },
        "LOCALS": { "offset": 20, "type": "u16" },
        "STACK": { "offset": 22, "type": "u16" },
        "CAPTURES": { "offset": 24, "type": "u16" },
        "RESERVED": { "offset": 26, "type": "u16" },
        "CODE": { "offset": 28, "type": "Ref" },
        "CONSTS": { "offset": 32, "type": "Ref" },
        "CHILDREN": { "offset": 36, "type": "Ref" },
        "CAPTURE_DESC": { "offset": 40, "type": "Ref" },
        "HANDLERS": { "offset": 44, "type": "Ref" },
        "LINES": { "offset": 48, "type": "Ref" },
        "SCRIPT": { "offset": 52, "type": "Ref" },
        "SRC_START": { "offset": 56, "type": "u32" },
        "SRC_END": { "offset": 60, "type": "u32" },
        "NAMES": { "offset": 64, "type": "Ref" },
        "RESERVED2": { "offset": 68, "type": "u32" }
      },
      "tail": null
    },
    "Script": {
      "kind": 13,
      "minSize": 16,
      "fields": {
        "NAME": { "offset": 8, "type": "Ref" },
        "SOURCE": { "offset": 12, "type": "Ref" }
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
    "Interp": {
      "kind": 17,
      "minSize": 48,
      "fields": {
        "STATE": { "offset": 8, "type": "u32" },
        "BP": { "offset": 12, "type": "u32" },
        "HP": { "offset": 16, "type": "u32" },
        "SP": { "offset": 20, "type": "u32" },
        "PC": { "offset": 24, "type": "u32" },
        "HOST_CALL": { "offset": 28, "type": "u32" },
        "NEXT_CALL_ID": { "offset": 32, "type": "u32" },
        "FLAGS": { "offset": 36, "type": "u32" },
        "RESULT": { "offset": 40, "type": "Value" }
      },
      "tail": null
    },
    "Realm": {
      "kind": 18,
      "minSize": 16,
      "fields": {
        "GLOBAL_LEX": { "offset": 8, "type": "Ref" }
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
      "minSize": 32,
      "fields": {
        "ID": { "offset": 8, "type": "u32" },
        "FN": { "offset": 12, "type": "u32" },
        "ARGS": { "offset": 16, "type": "Ref" },
        "THIS": { "offset": 24, "type": "Value" }
      },
      "tail": null
    }
  }
} as const;
