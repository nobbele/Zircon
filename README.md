# Zircon

A slightly higher level assembly language for the Z80. This is meant to alleviate some of the pains of using assembly, but not going so high level as C for performance reasons. The plan is to use this to write an operating system.

## Syntax

Below is a simple example of Zircon code.

```
sub boot {
    // Loading value by immediate
    ld A, $FF
    // By variable address
    ld A, &my_var
    // By variable
    ld B, my_var

    // Storing values by address
    ld $6000*, A
    // By constant
    ld some_constant*, A

    jp boot
    fallthrough
}
```

Addresses (also known as "pointers") are specified by a `*` at the end of the number, to differentiate from an immediate. Other assembly languages use `ld (some_constant), A` or `mov [some_constant], A`.

A subroutine block requires that the programmer writes a jump (`jmp` or some conditional variant), `ret`, `hlt` or the special keyword `fallthrough` to ignore any safety checks and possibly run whatever lies past the block in memory.

## Goals
- Be almost as low-level as normal assembly
- Make it faster and safer to write assembly
- Make it easier to organize assembly projects
- Provide good error handling

## Optional goals
- Support multiple CPU targets (with different assembly instructions of course)

## Non-goals
- Not be as high-level as C
- Not be a file format in itself. Which means the programmer has to for example, set memory addresses themselves

## Todo
- [x] Subroutine blocks
- [ ] Memory constants (in the ROM)
- [ ] Variables (in the RAM)
- [ ] Using blocks (temporary register aliases)
- [ ] If blocks
- [ ] Origin pragmas (for specifying addresses in the ROM)
- [ ] Multiple modules
- [ ] Complete instruction set
- [ ] Write-checker to require annotation of register modifications on subroutines

## Credit
Thanks to [Canop](https://github.com/Canop) who wrote the [char_reader](https://crates.io/crates/char_reader) crate that I used for my own version of `CharReader`