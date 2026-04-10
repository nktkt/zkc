use std::mem;

use crate::ast::{
    BinaryOp, Circuit, ConstrainStmt, ExposeStmt, Expr, InputDecl, Item, LetStmt, Program,
    Statement, Type, UnaryOp, Visibility,
};
use crate::error::{CompileError, CompileResult};
use crate::lexer::{Token, TokenKind, lex};
use crate::span::Span;

pub fn parse(source: &str) -> CompileResult<Program> {
    let tokens = lex(source)?;
    Parser::new(tokens).parse_program()
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

        let mut items = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.check(&TokenKind::Public) || self.check(&TokenKind::Private) {
                items.push(Item::Input(self.parse_input_decl()?));
            } else {
                items.push(Item::Statement(self.parse_statement()?));
            }
        }

        self.expect_simple(TokenKind::RBrace, "expected `}` at end of circuit body")?;
        self.expect_simple(TokenKind::Eof, "expected end of file")?;

        Ok(Program {
            circuit: Circuit { name, items },
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
        self.expect_simple(TokenKind::Field, "expected `field` type")?;
        self.expect_simple(TokenKind::Semicolon, "expected `;` after input declaration")?;

        Ok(InputDecl {
            visibility,
            name,
            ty: Type::Field,
            span: visibility_token.span,
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
            TokenKind::Ident(name) => Ok(Expr::Ident {
                name,
                span: token.span,
            }),
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
