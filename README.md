# funjit

A fun jit for befunge.

## Goals

- [x] befunge-93
- [x] tracing jit in the style of [befunjit](https://github.com/adrianton3/befunjit)
- [ ] direct stack manipulation instead of going through State::push/pop

## Running

```
$ cargo run -- examples/hello.bf
    Finished dev [unoptimized + debuginfo] target(s) in 0.00s
     Running `target/debug/funjit hello.bf`
Hello, world!

```
