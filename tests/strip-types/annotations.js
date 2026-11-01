// Annotations, generics, assertions.
let a         = 1, b         , c                               = [];
const d                            /* kept */ = { x: 1 };
function f                       (              x   , y         , ...rest          )    {
  return x;
}
function g(             ) {}
const h =     (x   )    => x;
const i = async    (x   )             => x;
const j = (x        )                => ({ r: x });
const k = a                     ;
const l = d.x  + (d.y          ).length;
const m = f          ({ a: 1 })                 ;
const n = new Map                ();
const o = f          `tpl`;
const p = f        ;
let q = a                            ;
let r          , s                , t              ;
let u                     , v                  , w                         ;
for (let z         = 0; z < 1; z++) {}
try {} catch (e         ) {}
const { x: dx }                = d;
function asserts(x         )                      {}
function pred(x         )              { return true; }
