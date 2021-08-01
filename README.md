# `potnet` and `potcpu`

Those two command line utilities are needed by the FreeBSD jail framework [`pot`](https://github.com/pizzamig/pot)

## `potnet`

`potnet` provides several features to manage the `public-bridge` and the `private-bridge` network types, like IPs allocation, network segmentation, IP validation and so on.

## `potcpu`
`potcpu` provides features to manage the `cpuset` based CPU allocation

## Installation

You can install `potnet` and `potcpu` via `pkg`:
```shell
# pkg install potnet
```

or via cargo:
```shell
$ cargo install potnet
```
