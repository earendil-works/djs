# Cell layouts

Every cell kind in the arena, drawn from `LAYOUTS` (the `cells!` declarations in
`crates/djs-core/src/heap/cells.rs`). The garbage collector traces exactly the orange
fields, and the host's reader (`js/src/layout.generated.ts`) is generated from the same
table.

<!-- gen:cell-all -->
## Roots (kind 1)

![The Roots cell](gen/cell-Roots.svg)

## Str8 (kind 2)

![The Str8 cell](gen/cell-Str8.svg)

## Str16 (kind 3)

![The Str16 cell](gen/cell-Str16.svg)

## Rope (kind 4)

![The Rope cell](gen/cell-Rope.svg)

## Slice (kind 22)

![The Slice cell](gen/cell-Slice.svg)

## Symbol (kind 5)

![The Symbol cell](gen/cell-Symbol.svg)

## Object (kind 6)

![The Object cell](gen/cell-Object.svg)

## PropTable (kind 7)

![The PropTable cell](gen/cell-PropTable.svg)

## ValueArray (kind 8)

![The ValueArray cell](gen/cell-ValueArray.svg)

## U32Array (kind 9)

![The U32Array cell](gen/cell-U32Array.svg)

## AtomTable (kind 10)

![The AtomTable cell](gen/cell-AtomTable.svg)

## HandleTable (kind 11)

![The HandleTable cell](gen/cell-HandleTable.svg)

## FuncTemplate (kind 12)

![The FuncTemplate cell](gen/cell-FuncTemplate.svg)

## Script (kind 13)

![The Script cell](gen/cell-Script.svg)

## BoxCell (kind 14)

![The BoxCell cell](gen/cell-BoxCell.svg)

## Bytes (kind 15)

![The Bytes cell](gen/cell-Bytes.svg)

## RefArray (kind 16)

![The RefArray cell](gen/cell-RefArray.svg)

## FuncInfo (kind 21)

![The FuncInfo cell](gen/cell-FuncInfo.svg)

## Interp (kind 17)

![The Interp cell](gen/cell-Interp.svg)

## Realm (kind 18)

![The Realm cell](gen/cell-Realm.svg)

## HostCall (kind 19)

![The HostCall cell](gen/cell-HostCall.svg)

## EphemeronArray (kind 20)

![The EphemeronArray cell](gen/cell-EphemeronArray.svg)

## AccessorPair (kind 25)

![The AccessorPair cell](gen/cell-AccessorPair.svg)

## Shape (kind 26)

![The Shape cell](gen/cell-Shape.svg)

## PropValues (kind 27)

![The PropValues cell](gen/cell-PropValues.svg)

## BigInt (kind 24)

![The BigInt cell](gen/cell-BigInt.svg)

<!-- /gen -->
