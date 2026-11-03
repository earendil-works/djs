# pi codemode fixtures

Copied verbatim from pi-private (read-only), for the pi codemode adapter (`js/pi/`, issue
0049) and its tests (`js/test/pi-vm.test.ts`):

| file | source | pi commit |
| --- | --- | --- |
| `prelude-source.ts` | `packages/codemode/src/runtime/prelude-source.ts` | d65de9db942d28e35bcb7b7b53008c56919f9c33 |

Refresh with
`cp ~/Development/pi-private/packages/codemode/src/runtime/prelude-source.ts tests/pi/`
and update the commit above (`git -C ~/Development/pi-private rev-parse HEAD`). pi's own
tests run against the adapter with `node js/pi/pi-tests.ts`, which copies them fresh each
time (docs/host.md "pi codemode").

`prelude-next.ts` is not a copy: it is pi's prelude changed into the shape pi should move
to on djs (D2565, D2650), with every difference listed at its top. js/pi/vm.ts runs it in
its `next` mode (`shape: "next"`). When `prelude-source.ts` is refreshed, carry pi's
changes over into it.
