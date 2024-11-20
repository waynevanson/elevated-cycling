# `range-split`

A subset of the UNIX `split` command built with a PRNG for creating files of differening and deterministic sizes, seeded from the input.

## Quick start

## Use cases

### Hosting large files in a repository

There's a bit of fluffling around but this can be used to host large files in GitHub without them knowing.

```sh
#  archive, compress, split

INPUT_DIR="path/to/big/directory"
PREFIX=".parts/"
RANGE="(0KB,50MB)" # 50MB is GitHub recommended max size.
tar cf - "$INPUT_DIR" | xz -9 -c | range-split -vvvv --factor 16 --prefix "$PREFIX" "$RANGE"
```

```sh
# concatenate, uncompress, unarchive

cat "$@" | xz -d -c | tar xf -
```

## Nix (Flakes)

This library is available as a flake.

```nix
{
    inputs = {
        range-split.url = "github:waynevanson/elevated-cycling";
    };

    outputs = {}: {

    };
}
```
