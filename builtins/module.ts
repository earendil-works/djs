// ES modules (0241, D3171): the evaluation of a linked module graph and the traps of
// module namespace objects. Loading and linking are leaf work in Rust (interp/module.rs);
// evaluation calls the module bodies, so it is here.
//
// A module record is an internal Record (interp/module.rs has the layout): slot 1 the
// status (2 linked, 3 evaluating, 4 evaluated, 5 evaluated with the error in slot 5),
// 2 the module body, 3 the requested modules (a Record of records) and 4 their count, 6
// and 7 [[DFSIndex]] and [[DFSAncestorIndex]], 10 the record under it on the evaluation
// stack.
//
// @modules-only: left out of the preludes of builds without ES modules (D3190).

/**
 * Evaluate ( ), 16.2.1.5.3, of a linked module, without top-level await: the promise of
 * the evaluation, settled when it returns. An error is kept by every module on the stack
 * and thrown again by later evaluations.
 * @intrinsic
 */
function ModuleEvaluate(m: Obj): Obj {
  const promise: Obj = PromiseCreate(RealmIntrinsic("PromisePrototype"));
  // [the top of the stack, the next DFS index]
  const state: Obj = NewRecord(2);
  RecordSet(state, 0, null);
  RecordSet(state, 1, 0);
  try {
    InnerModuleEvaluation(m, state);
  } catch (e) {
    let top: Value = RecordGet(state, 0);
    while (top !== null) {
      const t: Obj = AsObj(top);
      RecordSet(t, 1, 5);
      RecordSet(t, 5, e);
      top = RecordGet(t, 10);
      RecordSet(t, 10, null);
    }
    RejectPromise(promise, e);
    return promise;
  }
  FulfillPromise(promise, undefined);
  return promise;
}

/** InnerModuleEvaluation ( module, stack, index ), 16.2.1.5.3.1, without async modules. */
function InnerModuleEvaluation(m: Obj, state: Obj): void {
  const status: f64 = AsNumber(RecordGet(m, 1));
  if (status === 5) {
    throw RecordGet(m, 5);
  }
  if (status !== 2) {
    // Evaluating (a cycle) or evaluated.
    return;
  }
  const index: f64 = AsNumber(RecordGet(state, 1));
  RecordSet(m, 1, 3);
  RecordSet(m, 6, index);
  RecordSet(m, 7, index);
  RecordSet(state, 1, index + 1);
  RecordSet(m, 10, RecordGet(state, 0));
  RecordSet(state, 0, m);
  const requested: Obj = AsObj(RecordGet(m, 3));
  const n: f64 = AsNumber(RecordGet(m, 4));
  for (let i: f64 = 0; i < n; i++) {
    const r: Obj = AsObj(RecordGet(requested, F64ToI32(i)));
    InnerModuleEvaluation(r, state);
    if (AsNumber(RecordGet(r, 1)) === 3) {
      const ancestor: f64 = AsNumber(RecordGet(r, 7));
      if (ancestor < AsNumber(RecordGet(m, 7))) {
        RecordSet(m, 7, ancestor);
      }
    }
  }
  Call(RecordGet(m, 2), undefined);
  if (AsNumber(RecordGet(m, 7)) === index) {
    // The root of a strongly connected component: it and everything above it are done.
    let done: boolean = false;
    while (!done) {
      const t: Obj = AsObj(RecordGet(state, 0));
      RecordSet(state, 0, RecordGet(t, 10));
      RecordSet(t, 10, null);
      RecordSet(t, 1, 4);
      done = t === m;
    }
  }
}

// A module namespace object (10.4.6) is a Proxy whose handler is an internal Record of
// the namespace (its prototype holds these traps) and whose target is a non-extensible
// object with the same keys, so that the proxy invariants hold. The traps read the
// bindings; the others (has, deleteProperty, getPrototypeOf, ...) are the target's,
// which answers them as a namespace does.

/** [[Get]] ( P, Receiver ), 10.4.6.8: the binding's value (TDZ: ReferenceError). @intrinsic */
function NamespaceGet(this: Value, target: Value, key: Value, receiver: Value): Value {
  return ModuleNamespace(0, AsObj(this), AsObj(target), key, undefined);
}

/** [[Set]] ( P, V, Receiver ), 10.4.6.9: always false. @intrinsic */
function NamespaceSet(this: Value, target: Value, key: Value, v: Value, receiver: Value): boolean {
  return false;
}

/** [[GetOwnProperty]] ( P ), 10.4.6.5. @intrinsic */
function NamespaceGetOwnProperty(this: Value, target: Value, key: Value): Value {
  return ModuleNamespace(1, AsObj(this), AsObj(target), key, undefined);
}

/** [[DefineOwnProperty]] ( P, Desc ), 10.4.6.6. @intrinsic */
function NamespaceDefineProperty(this: Value, target: Value, key: Value, desc: Value): boolean {
  return AsBool(ModuleNamespace(2, AsObj(this), AsObj(target), key, desc));
}

/** [[OwnPropertyKeys]] ( ), 10.4.6.11: the exports in code unit order, then Symbol.toStringTag. @intrinsic */
function NamespaceOwnKeys(this: Value, target: Value): Obj {
  return AsObj(ModuleNamespace(3, AsObj(this), AsObj(target), undefined, undefined));
}
