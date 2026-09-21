use nexa_diagnostics::{CompileError, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Ident(String),
    Number(String),
    String(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Less,
    Greater,
    Colon,
    Dot,
    Semicolon,
    Comma,
    Equal,
    Plus,
    Minus,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>, CompileError> {
    let mut lexer = Lexer {
        source,
        offset: 0,
        line: 1,
        column: 1,
        tokens: Vec::new(),
    };
    lexer.run()?;
    Ok(lexer.tokens)
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn run(&mut self) -> Result<(), CompileError> {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.bump();
                continue;
            }
            if ch == '/' && self.peek_next() == Some('/') {
                while let Some(c) = self.bump() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            let start = self.mark();
            let kind = match ch {
                '{' => {
                    self.bump();
                    Kind::LBrace
                }
                '}' => {
                    self.bump();
                    Kind::RBrace
                }
                '(' => {
                    self.bump();
                    Kind::LParen
                }
                ')' => {
                    self.bump();
                    Kind::RParen
                }
                '[' => {
                    self.bump();
                    Kind::LBracket
                }
                ']' => {
                    self.bump();
                    Kind::RBracket
                }
                '<' => {
                    self.bump();
                    Kind::Less
                }
                '>' => {
                    self.bump();
                    Kind::Greater
                }
                ':' => {
                    self.bump();
                    Kind::Colon
                }
                '.' => {
                    self.bump();
                    Kind::Dot
                }
                ';' => {
                    self.bump();
                    Kind::Semicolon
                }
                ',' => {
                    self.bump();
                    Kind::Comma
                }
                '=' => {
                    self.bump();
                    Kind::Equal
                }
                '+' => {
                    self.bump();
                    Kind::Plus
                }
                '-' => {
                    self.bump();
                    Kind::Minus
                }
                '"' => Kind::String(self.string(start)?),
                c if c.is_ascii_digit() => Kind::Number(self.number()),
                c if is_ident_start(c) => Kind::Ident(self.identifier()),
                _ => {
                    return Err(CompileError::new(
                        start,
                        format!("unexpected character {ch:?}"),
                    ));
                }
            };
            self.tokens.push(Token {
                kind,
                span: self.span_from(start),
            });
        }
        let span = self.mark();
        self.tokens.push(Token {
            kind: Kind::Eof,
            span,
        });
        Ok(())
    }

    fn string(&mut self, start: Span) -> Result<String, CompileError> {
        self.bump();
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(other) => {
                        return Err(CompileError::new(
                            start,
                            format!("unsupported string escape \\{other}"),
                        ));
                    }
                    None => return Err(CompileError::new(start, "unterminated string escape")),
                },
                Some('\n') | None => {
                    return Err(CompileError::new(start, "unterminated string literal"));
                }
                Some(c) => out.push(c),
            }
        }
    }

    fn number(&mut self) -> String {
        let start = self.offset;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some('.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
        }
        self.source[start..self.offset].to_owned()
    }

    fn identifier(&mut self) -> String {
        let start = self.offset;
        self.bump();
        while self.peek().is_some_and(is_ident_continue) {
            self.bump();
        }
        self.source[start..self.offset].to_owned()
    }

    fn mark(&self) -> Span {
        Span {
            start: self.offset,
            end: self.offset,
            line: self.line,
            column: self.column,
        }
    }
    fn span_from(&self, start: Span) -> Span {
        Span {
            start: start.start,
            end: self.offset,
            ..start
        }
    }
    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }
    fn peek_next(&self) -> Option<char> {
        self.source[self.offset..].chars().nth(1)
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}
fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}
