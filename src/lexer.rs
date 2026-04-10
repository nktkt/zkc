use crate::error::{CompileError, CompileResult};
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Circuit,
    Public,
    Private,
    Let,
    Constrain,
    Expose,
    As,
    Field,
    Ident(String),
    Number(i128),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Semicolon,
    Equal,
    EqualEqual,
    Plus,
    Minus,
    Star,
    Eof,
}

pub fn lex(source: &str) -> CompileResult<Vec<Token>> {
    Lexer::new(source).tokenize()
}

struct Lexer {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn tokenize(mut self) -> CompileResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_ignored();
            let span = self.current_span();
            let Some(ch) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span,
                });
                break;
            };

            let kind = match ch {
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                ':' => {
                    self.advance();
                    TokenKind::Colon
                }
                ';' => {
                    self.advance();
                    TokenKind::Semicolon
                }
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '-' => {
                    self.advance();
                    TokenKind::Minus
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::EqualEqual
                    } else {
                        TokenKind::Equal
                    }
                }
                ch if is_ident_start(ch) => self.lex_identifier(),
                ch if ch.is_ascii_digit() => self.lex_number(span)?,
                other => {
                    return Err(CompileError::new(
                        span,
                        format!("unexpected character `{other}`"),
                    ));
                }
            };

            tokens.push(Token { kind, span });
        }

        Ok(tokens)
    }

    fn skip_ignored(&mut self) {
        loop {
            while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
                self.advance();
            }

            if self.peek() == Some('#') {
                self.skip_comment();
                continue;
            }

            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                self.skip_comment();
                continue;
            }

            break;
        }
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '\n' {
                break;
            }
        }
    }

    fn lex_identifier(&mut self) -> TokenKind {
        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if !is_ident_continue(ch) {
                break;
            }
            ident.push(ch);
            self.advance();
        }

        match ident.as_str() {
            "circuit" => TokenKind::Circuit,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "let" => TokenKind::Let,
            "constrain" => TokenKind::Constrain,
            "expose" => TokenKind::Expose,
            "as" => TokenKind::As,
            "field" => TokenKind::Field,
            _ => TokenKind::Ident(ident),
        }
    }

    fn lex_number(&mut self, span: Span) -> CompileResult<TokenKind> {
        let mut literal = String::new();
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_digit() {
                break;
            }
            literal.push(ch);
            self.advance();
        }

        let parsed = literal.parse::<i128>().map_err(|err| {
            CompileError::new(span, format!("invalid integer literal `{literal}`: {err}"))
        })?;
        Ok(TokenKind::Number(parsed))
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.index += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn current_span(&self) -> Span {
        Span::new(self.line, self.column)
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
