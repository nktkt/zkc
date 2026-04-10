# Builtins

`zkc` provides a small built-in function catalog. Builtins are available without declaration and are
expanded by the compiler during typechecking.

## Design Notes

- Builtins are pure.
- Builtins are typechecked before expansion.
- Builtin names are reserved and cannot be redefined by user code.
- Builtins are expanded into ordinary typed expressions before arithmetic IR lowering.

## Catalog

- `square(value)` returns `value * value`
- `cube(value)` returns `value * value * value`
- `double(value)` returns `value + value`
- `triple(value)` returns `value + value + value`
- `quad(value)` returns four copies of `value` added together
- `negate(value)` returns `-value`
- `sum2(a, b)` returns `a + b`
- `sum3(a, b, c)` returns `a + b + c`
- `sum4(a, b, c, d)` returns `a + b + c + d`
- `mul_add(a, b, c)` returns `a * b + c`
- `blend2(a, wa, b, wb)` returns `a * wa + b * wb`
- `weighted_sum3(a, wa, b, wb, c, wc)` returns `a * wa + b * wb + c * wc`
- `not(value)` returns the boolean negation of `value`
- `and(lhs, rhs)` returns the boolean conjunction of `lhs` and `rhs`
- `or(lhs, rhs)` returns the boolean disjunction of `lhs` and `rhs`
- `xor(lhs, rhs)` returns the boolean exclusive-or of `lhs` and `rhs`
- `choose(cond, when_true, when_false)` returns one of two `field` values based on a `bool`
- `choose_bool(cond, when_true, when_false)` returns one of two `bool` values based on a `bool`

## Example

```zk
circuit builtins_demo {
    public expected: field;
    private a: field;
    private b: field;
    private c: field;

    let left = square(a);
    let middle = mul_add(b, 3, 1);
    let right = weighted_sum3(a, 2, b, 3, c, 4);
    let result = sum3(left, middle, right);
    constrain result == expected;
    expose result;
}
```

## Boolean Example

```zk
circuit bool_builtins_demo {
    public expected: bool;
    private a: bool;
    private b: bool;

    let result = xor(or(a, b), and(a, b));
    constrain result == expected;
    expose result;
}
```
