# zkc Specification v0

## Purpose

`zkc` is a small domain-specific language for describing arithmetic constraint systems over a prime
field. Version 0 is intentionally minimal and is designed to be understandable, testable, and easy
to evolve.

This version does not define proof generation. It defines source syntax, name resolution, lowering
semantics, and witness evaluation semantics.

## Core Model

A `zkc` program defines exactly one circuit. A circuit consists of:

- optional include directives
- optional import directives
- public inputs
- pure helper functions
- private inputs
- local bindings
- equality constraints
- public outputs

`field` values live in a single prime field with modulus `18446744073709551557`.

`bool` values are represented in the lowered circuit as field elements constrained to `0` or `1`.

## Syntax

```text
program        := "circuit" IDENT "{" item* "}"
item           := include_decl | import_decl | input_decl | function_decl | stmt
include_decl   := "include" STRING ";"
import_decl    := "import" STRING "as" IDENT ";"
input_decl     := ("public" | "private") IDENT ":" type ";"
function_decl  := "fn" IDENT "(" params? ")" "->" type "{" expr "}"
params         := param ("," param)*
param          := IDENT ":" type
type           := "field" | "bool"
stmt           := let_stmt | constrain_stmt | expose_stmt
let_stmt       := "let" IDENT "=" expr ";"
constrain_stmt := "constrain" expr "==" expr ";"
expose_stmt    := "expose" expr ("as" IDENT)? ";"
expr           := term (("+" | "-") term)*
term           := unary ("*" unary)*
unary          := "-" unary | primary
primary        := NUMBER | "true" | "false" | IDENT | call | if_expr | "(" expr ")"
if_expr        := "if" expr "{" expr "}" "else" "{" expr "}"
call           := callee "(" args? ")"
callee         := IDENT ("::" IDENT)*
args           := expr ("," expr)*
```

Comments are supported in two forms:

- line comments starting with `#`
- line comments starting with `//`

String literals are currently only supported for `include` and `import` paths.

## Names and Scope

- Include directives are resolved before name resolution.
- Import directives are also resolved before name resolution.
- Include paths are interpreted relative to the including file.
- Included items are spliced into the circuit item stream at the include site.
- Imported functions are rewritten into qualified names such as `alias::name`.
- Each input declaration introduces a name into the circuit scope.
- Each function declaration introduces a function name into the circuit scope.
- Each `let` statement introduces a name into the circuit scope after its initializer expression.
- Binding names and function names must be unique.
- A binding or function must be declared before it is referenced.
- There is no nested scope in v0.

## Types

Version 0 defines two scalar types:

- `field`
- `bool`

Type rules:

- arithmetic operators require `field` operands and return `field`
- boolean builtins require `bool` operands and return `bool`
- `if` conditions require `bool`
- both branches of an `if` expression must have the same type
- `constrain lhs == rhs;` requires both sides to have the same type

## Expression Semantics

- Integer literals are parsed as signed decimal values and reduced modulo the field prime.
- Boolean literals are written as `true` and `false`.
- Unary negation computes the additive inverse in the field.
- `+`, `-`, and `*` are field operations.
- `if cond { a } else { b }` evaluates `cond` as a boolean selector and returns either branch.
- Parentheses only affect grouping.

## Constraint Semantics

`constrain lhs == rhs;` requires the two expressions to evaluate to the same typed value.

During witness evaluation, a constraint failure terminates execution with an error.

## Function Semantics

- Functions are pure helpers with expression bodies.
- Functions may accept `field` and `bool` parameters and may return either supported scalar type.
- Functions do not capture circuit-level bindings.
- Functions may call previously declared functions.
- Recursive functions are not supported in v0.

## Output Semantics

- `expose expr;` exposes the expression under an inferred output name.
- `expose expr as name;` exposes the expression under the explicit output name.
- If the exposed expression is a simple identifier and no alias is provided, the identifier name is
  used.
- Otherwise the compiler generates a synthetic output label.

## Lowering Model

The compiler lowers a program into a wire-based arithmetic circuit IR with:

- named public input wires
- named private input wires
- arithmetic operations `add`, `sub`, and `mul`
- booleanity constraints for `bool` inputs
- equality constraints
- named outputs

Literal-only arithmetic expressions may be constant-folded during lowering. Boolean conditionals and
builtins lower to arithmetic combinations over boolean selector values.

## Include Resolution

Version 0 defines file-based composition through `include "path.zk";`.

Resolution rules:

- include resolution occurs before typechecking
- included files are parsed as item fragments, not full circuits
- nested includes are allowed
- `@std/...` resolves into the repository `stdlib/` directory
- already-expanded include files are skipped on later includes
- circular includes are rejected
- dependency graphs can be emitted by the CLI

## Import Resolution

Version 0 also defines namespaced source-level modules through `import "path.zk" as alias;`.

Resolution rules:

- import resolution occurs before typechecking
- imported files are parsed as item fragments, not full circuits
- imported files may contain functions, includes, and nested imports
- imported files may not contain inputs or top-level statements
- imported functions are namespaced under the chosen alias
- nested imports extend the namespace, such as `left::select::helper`
- repeated imports of the same file are allowed when the namespace differs
- cycles are rejected using the same path-stack checks as includes

## Runtime Inputs

The CLI accepts witness assignments as:

```text
--public name=value
--private name=value
```

Runtime behavior:

- undeclared assignments are rejected
- missing declared inputs are rejected
- CLI values `true` and `false` are accepted as aliases for `1` and `0`
- constraints are checked after all operations are evaluated

## Errors

Version 0 surfaces human-readable diagnostics for:

- unexpected characters
- syntax errors
- duplicate declarations
- use of undeclared identifiers
- type mismatches in function calls and conditionals
- missing runtime inputs
- unexpected runtime inputs
- constraint failures

## Non-Goals

Version 0 does not define:

- proof generation
- proof verification
- richer types such as arrays or structs
- package manifests
- typed module interfaces
- loops or statement-level control flow
- package management
- optimization beyond basic constant folding
