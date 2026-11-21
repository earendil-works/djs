#!/bin/bash
# famvar.sh NAME FILE...: the base image's size with only FILE... left out of the full prelude
# (the others' @slim-omit markers stripped); writes base-NAME.img.
set -e
W=/Users/mitsuhiko/Development/djs/.worktrees/pstart
S=/Users/mitsuhiko/Development/djs/work/scratch/0283
name=$1; shift
rm -rf $S/bvar && mkdir -p $S/bvar && cp -R $W/builtins $S/bvar/
[ -n "$STUB" ] && cp $STUB $S/bvar/builtins/
for f in $S/bvar/builtins/*.ts; do
  keep=0; for g in "$@"; do [ "$(basename $f)" = "$g" ] && keep=1; done
  if [ $keep = 1 ] && ! grep -q "@slim-omit" $f; then printf '// @slim-omit: experiment\n' | cat - $f > $f.tmp && mv $f.tmp $f; fi
  if [ $keep = 0 ]; then sed -i '' -e 's|^// @slim-omit.*$|//|' -e 's|^ \* @slim-omit$| *|' $f; fi
done
$W/target/release/djs-bgen --dir $S/bvar/builtins
# the slim prelude of this copy = the full one minus FILE...; drop the embed-only files too
cp $W/builtins/gen/prelude-noembed.js $S/prelude-noembed.orig
cp $S/bvar/builtins/gen/prelude-slim.js $W/builtins/gen/prelude-noembed.js
trap "cp $S/prelude-noembed.orig $W/builtins/gen/prelude-noembed.js" EXIT
cd $W && CARGO_TARGET_DIR=target/basevar nice -n 10 cargo build -q -p djs-prelude --no-default-features --features raw,full 2>&1 | grep -E "error|panicked" | head -5
img=$(ls -t target/basevar/debug/build/djs-prelude-*/out/base.img | head -1)
cp $img $S/base-$name.img
echo "$name $(stat -f %z $img)"
