use std::convert::Infallible;
use std::env;
use std::fs;
use std::iter::Peekable;

mod options;
mod peek_while;

use options::Options;
use peek_while::peek_while;

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct Expression {
	kind: ExpressionKind,
	values: Vec<Phrase>,
}

impl Expression {
	pub fn null(value: Phrase) -> Self {
		Self {
			kind: ExpressionKind::Null,
			values: vec![value],
		}
	}
}

#[derive(Clone, Debug)]
enum ExpressionKind {
	Block,
	List,
	Item,
	Null,
}

#[derive(Clone, Debug)]
enum Phrase {
	Expression(Expression),
	Identifier(String),
	Text(String),
	Number(String),
	Comment(String),
}

#[derive(Clone, Debug)]
enum ParseError {
	OhShit,
}

impl From<Infallible> for ParseError {
	fn from(_: Infallible) -> Self {
		unreachable!()
	}
}

struct Parser<'a, I>
where
	I: Iterator<Item = char>,
{
	s: &'a mut Peekable<I>,
	row: u32,
	col: u32,
}

impl<'a, I: Iterator<Item = char>> Iterator for Parser<'a, I> {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		self.col += 1;
		self.s.next()
	}
}

impl<'a, I: Iterator<Item = char>> Parser<'a, I> {
	pub fn new(stream: &'a mut Peekable<I>) -> Self {
		Self {
			row: 0,
			col: 0,
			s: stream,
		}
	}

	pub fn peek(&mut self) -> Option<&char> {
		self.s.peek()
	}
}

fn parse_whitespace(s: &mut Parser<impl Iterator<Item = char>>) -> Result<(), Infallible> {
	while let Some(c) = s.peek() {
		if !c.is_whitespace() {
			break;
		}
		s.next();
	}

	Ok(())
}

fn parse_string(s: &mut Parser<impl Iterator<Item = char>>) -> Result<Phrase, ParseError> {
	// Consume quote
	assert_eq!(s.next(), Some('"'));

	let mut is_next_escaped = false;

	let text = s
		.take_while(|&c| {
			if is_next_escaped {
				is_next_escaped = false;
				return true;
			}

			if c == '\\' {
				is_next_escaped = true;
				return true;
			}

			c != '"'
		})
		.collect();

	Ok(Phrase::Text(text))
}

fn parse_comment(s: &mut Parser<impl Iterator<Item = char>>) -> Result<Phrase, ParseError> {
	assert_eq!(s.next(), Some(';'));
	let body = s.take_while(|&c| c != '\n').collect();
	Ok(Phrase::Comment(body))
}

fn parse_number(s: &mut Parser<impl Iterator<Item = char>>) -> Result<Phrase, ParseError> {
	let mut contains_point = false;

	let number = peek_while(s.s, |&c: &char| {
		if !contains_point && c == '.' {
			contains_point = true;
			return true;
		}

		c.is_ascii_digit()
	})
	.collect();

	Ok(Phrase::Number(number))
}

fn parse_identifier(s: &mut Parser<impl Iterator<Item = char>>) -> Result<Phrase, ParseError> {
	let identifier = peek_while(s.s, |&c| c.is_ascii_alphanumeric() || c == '_').collect();

	Ok(Phrase::Identifier(identifier))
}

fn parse_phrase(s: &mut Parser<impl Iterator<Item = char>>) -> Result<Phrase, ParseError> {
	parse_whitespace(s)?;

	match s.peek().ok_or(ParseError::OhShit)? {
		'(' | '[' | '{' => parse_expression(s).map(Phrase::Expression),
		'"' => parse_string(s),
		';' => parse_comment(s),
		x if x.is_ascii_digit() => parse_number(s),
		x if x.is_ascii_alphabetic() => parse_identifier(s),
		_ => Err(ParseError::OhShit),
	}
}

fn parse_expression(s: &mut Parser<impl Iterator<Item = char>>) -> Result<Expression, ParseError> {
	parse_whitespace(s)?;

	if s.peek() == Some(&';') {
		return Ok(Expression::null(parse_comment(s)?));
	}

	let kind = match s.next().ok_or(ParseError::OhShit)? {
		'[' => ExpressionKind::List,
		'{' => ExpressionKind::Block,
		'(' => ExpressionKind::Item,
		c => unreachable!("unexpected character {}", c),
	};

	let mut values = Vec::new();

	while let Ok(phrase) = parse_phrase(s) {
		values.push(phrase);
	}

	match kind {
		ExpressionKind::Block => assert_eq!(s.next(), Some('}')),
		ExpressionKind::List => assert_eq!(s.next(), Some(']')),
		ExpressionKind::Item => assert_eq!(s.next(), Some(')')),
		ExpressionKind::Null => unreachable!(),
	}

	Ok(Expression { kind, values })
}

fn parse_program(
	s: &mut Parser<impl Iterator<Item = char>>,
) -> Result<Vec<Expression>, ParseError> {
	let mut program = Vec::new();
	parse_whitespace(s)?;

	while s.peek().is_some() {
		program.push(parse_expression(s)?);
		parse_whitespace(s)?;
	}

	Ok(program)
}

fn main() -> Result<(), ParseError> {
	let options = env::args().skip(1).collect::<Options>();

	let source = fs::read_to_string(options.input).unwrap();
	let mut stream = source.chars().peekable();

	let mut parser = Parser::new(&mut stream);
	let program = parse_program(&mut parser).unwrap();

	if options.debug_parser {
		println!("{:?}", program);
	}

	Ok(())
}
