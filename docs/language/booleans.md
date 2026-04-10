# Booleans and Conditionals

`zkc` supports a small boolean layer on top of its arithmetic circuit model. Boolean values are
represented as field elements constrained to `0` or `1`, but the source language tracks them as a
distinct `bool` type.

## Available Boolean Features

- `bool` input and function parameter types
- `true` and `false` literals
- boolean-returning functions
- `if cond { then_expr } else { else_expr }` expressions
- boolean builtins: `not`, `and`, `or`, `xor`
- selector builtins: `choose` and `choose_bool`

## Boolean Inputs

Boolean inputs can be declared just like field inputs:

```zk
circuit gates {
    public enabled: bool;
    private local_flag: bool;

    expose enabled;
    expose local_flag;
}
```

At lowering time, each boolean input wire gets a booleanity constraint so the interpreter rejects
assignments other than `0` or `1`.

## Literals

Boolean literals are written as `true` and `false`:

```zk
circuit literals {
    let always = true;
    let never = false;
    expose always;
    expose never;
}
```

## Conditionals

Conditionals are expressions, not statements:

```zk
circuit selector {
    public gate: bool;
    private x: field;
    private y: field;

    let result = if gate { x + 5 } else { y + 1 };
    expose result;
}
```

Typing rules:

- the condition must have type `bool`
- both branches must have the same type
- the result type is the shared branch type

## Boolean Builtins

Boolean builtins allow common logical operations without exposing arithmetic encodings directly:

```zk
circuit logic {
    public expected: bool;
    private a: bool;
    private b: bool;

    let left = and(a, not(b));
    let right = xor(a, b);
    let result = or(left, right);

    constrain result == expected;
    expose result;
}
```

`choose` and `choose_bool` provide function-form selection helpers:

```zk
circuit choose_demo {
    public gate: bool;
    private x: field;
    private y: field;

    let result = choose(gate, x, y);
    expose result;
}
```

## CLI Input Values

The CLI accepts either numeric or boolean-style witness assignments:

```text
--public gate=true
--private local_flag=false
```

These values are converted to field elements `1` and `0`.

## Current Constraints

- arithmetic operators still only accept `field` operands
- there is no implicit cast between `bool` and `field`
- booleans can be exposed, but outputs are currently rendered as `0` or `1`
- there are no comparison operators yet
