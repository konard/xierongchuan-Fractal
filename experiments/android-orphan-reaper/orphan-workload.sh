#!/bin/sh
# Stands in for the `ninja` that `pixiewood build` launches.
#
# It reproduces the shape of the real failure: a process below `ninja` (there,
# a `cargo`) dies without waiting for the process it spawned (there, a
# `rustc`), so that grandchild is reparented to PID 1 of the container while
# `ninja` itself keeps going.
set -eu

# The inner shell backgrounds a child and then kills itself, orphaning it.
sh -c 'sleep 1 & kill -9 $$' || true

# `ninja` is still building when the orphan is reaped.
sleep 4
