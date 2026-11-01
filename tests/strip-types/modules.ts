// Module syntax and type-only imports and exports.
import fs from "node:fs";
import type { Stats } from "node:fs";
import { readFile, type PathLike, type Mode } from "node:fs";
import * as path from "node:path";
import type * as os from "node:os";
import def, { type T1, a1 } from "./x.js";
import "./side.js";
export type { Stats };
export type * from "./types.js";
export { readFile, type PathLike };
export * from "./all.js";
export const value: number = 1;
export function fn<T>(x: T): T { return x; }
export function ov(): void;
export function ov(x?: number): void {}
export type Alias = number;
export interface Shape { area(): number }
export declare const declared: number;
export default class {}
export abstract class Abs {}
const meta = import.meta;
const lazy = import("./lazy.js");
