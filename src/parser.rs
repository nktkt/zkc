use std::mem;

use crate::ast::{
    BinaryOp, Circuit, ConstrainStmt, ExposeStmt, Expr, FunctionDecl, ImportDecl, IncludeDecl,
    InputDecl, Item, LetStmt, Param, Program, Statement, Type, UnaryOp, Visibility,
};
use crate::error::{CompileError, CompileResult};
use crate::lexer::{Token, TokenKind, lex};
use crate::span::Span;

pub fn parse(source: &str) -> CompileResult<Program> {
    let tokens = lex(source)?;
    Parser::new(tokens).parse_program()
}

pub fn parse_items(source: &str) -> CompileResult<Vec<Item>> {
    let tokens = lex(source)?;
    Parser::new(tokens).parse_item_list()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn parse_program(&mut self) -> CompileResult<Program> {
        self.expect_simple(TokenKind::Circuit, "expected `circuit` keyword")?;
        let name = self.expect_ident("expected circuit name")?;
        self.expect_simple(TokenKind::LBrace, "expected `{` after circuit name")?;

        let items = self.parse_until(TokenKind::RBrace)?;

        self.expect_simple(TokenKind::RBrace, "expected `}` at end of circuit body")?;
        self.expect_simple(TokenKind::Eof, "expected end of file")?;

        Ok(Program {
            circuit: Circuit { name, items },
        })
    }

    fn parse_item_list(&mut self) -> CompileResult<Vec<Item>> {
        let items = self.parse_until(TokenKind::Eof)?;
        self.expect_simple(TokenKind::Eof, "expected end of file")?;
        Ok(items)
    }

    fn parse_until(&mut self, terminal: TokenKind) -> CompileResult<Vec<Item>> {
        let mut items = Vec::new();
        while !self.check(&terminal) {
            items.push(self.parse_item()?);
        }
        Ok(items)
    }

    fn parse_item(&mut self) -> CompileResult<Item> {
        if self.check(&TokenKind::Include) {
            Ok(Item::Include(self.parse_include_decl()?))
        } else if self.check(&TokenKind::Import) {
            Ok(Item::Import(self.parse_import_decl()?))
        } else if self.check(&TokenKind::Public) || self.check(&TokenKind::Private) {
            Ok(Item::Input(self.parse_input_decl()?))
        } else if self.check(&TokenKind::Fn) {
            Ok(Item::Function(self.parse_function_decl()?))
        } else {
            Ok(Item::Statement(self.parse_statement()?))
        }
    }

    fn parse_include_decl(&mut self) -> CompileResult<IncludeDecl> {
        let start = self.expect_simple(TokenKind::Include, "expected `include`")?;
        let token = self.advance().clone();
        let path = match token.kind {
            TokenKind::String(path) => path,
            _ => {
                return Err(CompileError::new(
                    token.span,
                    "expected string literal after `include`",
                ));
            }
        };
        self.expect_simple(TokenKind::Semicolon, "expected `;` after include path")?;
        Ok(IncludeDecl { path, span: start })
    }

    fn parse_import_decl(&mut self) -> CompileResult<ImportDecl> {
        let start = self.expect_simple(TokenKind::Import, "expected `import`")?;
        let token = self.advance().clone();
        let path = match token.kind {
            TokenKind::String(path) => path,
            _ => {
                return Err(CompileError::new(
                    token.span,
                    "expected string literal after `import`",
                ));
            }
        };
        self.expect_simple(TokenKind::As, "expected `as` after import path")?;
        let alias = self.expect_ident("expected import alias after `as`")?;
        self.expect_simple(
            TokenKind::Semicolon,
            "expected `;` after import declaration",
        )?;
        Ok(ImportDecl {
            path,
            alias,
            span: start,
        })
    }

    fn parse_input_decl(&mut self) -> CompileResult<InputDecl> {
        let visibility_token = self.advance().clone();
        let visibility = match visibility_token.kind {
            TokenKind::Public => Visibility::Public,
            TokenKind::Private => Visibility::Private,
            _ => {
                return Err(CompileError::new(
                    visibility_token.span,
                    "expected `public` or `private` input declaration",
                ));
            }
        };

        let name = self.expect_ident("expected input name")?;
        self.expect_simple(TokenKind::Colon, "expected `:` after input name")?;
        let ty = self.parse_type()?;
        self.expect_simple(TokenKind::Semicolon, "expected `;` after input declaration")?;

        Ok(InputDecl {
            visibility,
            name,
            ty,
            span: visibility_token.span,
        })
    }

    fn parse_function_decl(&mut self) -> CompileResult<FunctionDecl> {
        let start = self.expect_simple(TokenKind::Fn, "expected `fn`")?;
        let name = self.expect_ident("expected function name after `fn`")?;
        self.expect_simple(TokenKind::LParen, "expected `(` after function name")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let param_span = self.peek().span;
                let param_name = self.expect_ident("expected parameter name")?;
                self.expect_simple(TokenKind::Colon, "expected `:` after parameter name")?;
                let param_ty = self.parse_type()?;
                params.push(Param {
                    name: param_name,
                    ty: param_ty,
                    span: param_span,
                });

                if self.check(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }

                break;
            }
        }

        self.expect_simple(TokenKind::RParen, "expected `)` after parameter list")?;
        self.expect_simple(TokenKind::Arrow, "expected `->` after parameter list")?;
        let return_type = self.parse_type()?;
        self.expect_simple(TokenKind::LBrace, "expected `{` before function body")?;
        let body = self.parse_expr()?;
        self.expect_simple(TokenKind::RBrace, "expected `}` after function body")?;

        Ok(FunctionDecl {
            name,
            params,
            return_type,
            body,
            span: start,
        })
    }

    fn parse_statement(&mut self) -> CompileResult<Statement> {
        match &self.peek().kind {
            TokenKind::Let => self.parse_let_stmt().map(Statement::Let),
            TokenKind::Constrain => self.parse_constrain_stmt().map(Statement::Constrain),
            TokenKind::Expose => self.parse_expose_stmt().map(Statement::Expose),
            _ => Err(CompileError::new(
                self.peek().span,
                "expected `let`, `constrain`, or `expose`",
            )),
        }
    }

    fn parse_let_stmt(&mut self) -> CompileResult<LetStmt> {
        let start = self.expect_simple(TokenKind::Let, "expected `let`")?;
        let name = self.expect_ident("expected binding name after `let`")?;
        self.expect_simple(TokenKind::Equal, "expected `=` after binding name")?;
        let expr = self.parse_expr()?;
        self.expect_simple(TokenKind::Semicolon, "expected `;` after `let` binding")?;

        Ok(LetStmt {
            name,
            expr,
            span: start,
        })
    }

    fn parse_constrain_stmt(&mut self) -> CompileResult<ConstrainStmt> {
        let start = self.expect_simple(TokenKind::Constrain, "expected `constrain`")?;
        let lhs = self.parse_expr()?;
        self.expect_simple(TokenKind::EqualEqual, "expected `==` in constraint")?;
        let rhs = self.parse_expr()?;
        self.expect_simple(TokenKind::Semicolon, "expected `;` after constraint")?;

        Ok(ConstrainStmt {
            lhs,
            rhs,
            span: start,
        })
    }

    fn parse_expose_stmt(&mut self) -> CompileResult<ExposeStmt> {
        let start = self.expect_simple(TokenKind::Expose, "expected `expose`")?;
        let expr = self.parse_expr()?;
        let label = if self.check(&TokenKind::As) {
            self.advance();
            Some(self.expect_ident("expected output label after `as`")?)
        } else {
            None
        };
        self.expect_simple(
            TokenKind::Semicolon,
            "expected `;` after exposed expression",
        )?;

        Ok(ExposeStmt {
            expr,
            label,
            span: start,
        })
    }

    fn parse_expr(&mut self) -> CompileResult<Expr> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> CompileResult<Expr> {
        let mut expr = self.parse_multiplicative()?;

        loop {
            let op = if self.check(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.check(&TokenKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };
            let operator_span = self.advance().span;
            let rhs = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: operator_span,
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> CompileResult<Expr> {
        let mut expr = self.parse_unary()?;

        while self.check(&TokenKind::Star) {
            let operator_span = self.advance().span;
            let rhs = self.parse_unary()?;
            expr = Expr::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: operator_span,
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> CompileResult<Expr> {
        if self.check(&TokenKind::Minus) {
            let span = self.advance().span;
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
                span,
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> CompileResult<Expr> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number(value) => Ok(Expr::Number {
                value,
                span: token.span,
            }),
            TokenKind::True => Ok(Expr::Bool {
                value: true,
                span: token.span,
            }),
            TokenKind::False => Ok(Expr::Bool {
                value: false,
                span: token.span,
            }),
            TokenKind::Ident(name) => {
                if self.check(&TokenKind::LParen) || self.check(&TokenKind::ColonColon) {
                    let callee = self.parse_call_target(name)?;
                    let args = self.parse_call_args()?;
                    Ok(Expr::Call {
                        callee,
                        args,
                        span: token.span,
                    })
                } else {
                    Ok(Expr::Ident {
                        name,
                        span: token.span,
                    })
                }
            }
            TokenKind::If => {
                let condition = self.parse_expr()?;
                self.expect_simple(TokenKind::LBrace, "expected `{` before `if` branch")?;
                let then_branch = self.parse_expr()?;
                self.expect_simple(TokenKind::RBrace, "expected `}` after `if` branch")?;
                self.expect_simple(TokenKind::Else, "expected `else` after `if` branch")?;
                self.expect_simple(TokenKind::LBrace, "expected `{` before `else` branch")?;
                let else_branch = self.parse_expr()?;
                self.expect_simple(TokenKind::RBrace, "expected `}` after `else` branch")?;
                Ok(Expr::IfElse {
                    condition: Box::new(condition),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                    span: token.span,
                })
            }
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect_simple(TokenKind::RParen, "expected `)` after expression")?;
                Ok(expr)
            }
            _ => Err(CompileError::new(token.span, "expected expression")),
        }
    }

    fn expect_ident(&mut self, message: &str) -> CompileResult<String> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) => Ok(name),
            _ => Err(CompileError::new(token.span, message)),
        }
    }

    fn expect_simple(&mut self, expected: TokenKind, message: &str) -> CompileResult<Span> {
        if self.check(&expected) {
            return Ok(self.advance().span);
        }

        Err(CompileError::new(self.peek().span, message))
    }

    fn check(&self, expected: &TokenKind) -> bool {
        mem::discriminant(&self.peek().kind) == mem::discriminant(expected)
    }

    fn parse_type(&mut self) -> CompileResult<Type> {
        if self.check(&TokenKind::Field) {
            self.advance();
            Ok(Type::Field)
        } else if self.check(&TokenKind::Bool) {
            self.advance();
            Ok(Type::Bool)
        } else if let TokenKind::Ident(name) = &self.peek().kind {
            let ty = match name.as_str() {
                "u8" => Some(Type::U8),
                "u16" => Some(Type::U16),
                "u32" => Some(Type::U32),
                _ => None,
            };
            if let Some(ty) = ty {
                self.advance();
                Ok(ty)
            } else {
                Err(CompileError::new(
                    self.peek().span,
                    "expected `field`, `bool`, `u8`, `u16`, or `u32` type",
                ))
            }
        } else {
            Err(CompileError::new(
                self.peek().span,
                "expected `field`, `bool`, `u8`, `u16`, or `u32` type",
            ))
        }
    }

    fn parse_call_target(&mut self, first: String) -> CompileResult<Vec<String>> {
        let mut segments = vec![first];
        while self.check(&TokenKind::ColonColon) {
            self.advance();
            segments.push(self.expect_ident("expected identifier after `::`")?);
        }
        if !self.check(&TokenKind::LParen) {
            return Err(CompileError::new(
                self.peek().span,
                "qualified identifiers are only supported in call position",
            ));
        }
        self.advance();
        Ok(segments)
    }

    fn parse_call_args(&mut self) -> CompileResult<Vec<Expr>> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.expect_simple(TokenKind::RParen, "expected `)` after argument list")?;
        Ok(args)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.cursor];
        if !matches!(token.kind, TokenKind::Eof) {
            self.cursor += 1;
        }
        token
    }
}
