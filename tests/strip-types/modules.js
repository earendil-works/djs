// Module syntax and type-only imports and exports.
import fs from "node:fs";
                                     
import { readFile,                          } from "node:fs";
import * as path from "node:path";
                                   
import def, {          a1 } from "./x.js";
import "./side.js";
                      
                                
export { readFile,               };
export * from "./all.js";
export const value         = 1;
export function fn   (x   )    { return x; }
                           
export function ov(x         )       {}
;                          
;                                        
;                                     
export default class {}
export          class Abs {}
const meta = import.meta;
const lazy = import("./lazy.js");
