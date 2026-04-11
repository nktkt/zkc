# Integers

`zkc` now supports range-checked unsigned scalar types:

- `u8`
- `u16`
- `u32`

## Model

These integer values are represented internally as field elements plus explicit range assertions.

That means:

- they can be used as typed inputs
- they can flow through functions and `if` expressions
- they support checked arithmetic with the same integer width
- they can be compared with `constrain lhs == rhs`
- they can be exposed as public outputs

## Casts

Use the explicit cast builtins to reinterpret field expressions as range-checked integers:

```zk
let byte = into_u8(raw);
let short = into_u16(raw);
let word = into_u32(raw);
```

Convert back into a field expression with:

```zk
let value = into_field(byte);
```

## Arithmetic

Native integer arithmetic is now available for matching widths:

```zk
let bumped = byte + 1;
let mixed = bumped * 2 - 3;
```

The result stays in the same integer type and receives a new range assertion. Overflow or underflow
causes runtime failure through that range check.

## Current Limitation

Mixing integers and fields still requires an explicit conversion:

```zk
let mixed = into_field(byte) + field_value;
```
