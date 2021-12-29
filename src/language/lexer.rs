mod ast;
mod source;
mod token_kind;
use crate::ast::Token;
use crate::source::Source;
use crate::token_kind::TokenKind;

/// Given a Source object, creates a Lexer for that source.
/// A Lexer is a stateful stream generator in that every time
/// it is advanced, it returns the next token in the Source. Assuming the
/// source lexes, the final Token emitted by the lexer will be of kind
/// EOF, after which the lexer will repeatedly return the same EOF token
/// whenever called.
pub struct Lexer {
	source: Source,
	/// The previously focused non-ignored token.
	last_token: Token,
	/// The currently focused non-ignored token.
	token: Token,
	/// The (1-indexed) line containing the current token.
	line: usize,
	/// The character offset at which the current line begins.
	line_start: usize,
}

/// A Unicode scalar value is any Unicode code point except surrogate code
/// points. In other words, the inclusive ranges of values 0x0000 to 0xD7FF and
/// 0xE000 to 0x10FFFF.
///
/// SourceCharacter ::
///  - "Any Unicode scalar value"
///
fn isUnicodeScalarValue(code: usize) -> boolean {
	return ((code >= 0x0000 && code <= 0xd7ff) || (code >= 0xe000 && code <= 0x10ffff));
}

/// Reads an alphanumeric + underscore name from the source.
///
/// ```
/// Name ::
///   - NameStart NameContinue* [lookahead != NameContinue]
/// ```
// fn read_name(lexer: Lexer, start: usize)-> Token {
// 	let body = lexer.source.body;
// 	let bodyLength = body.len();
// 	let position = start + 1;
// 	while (position < bodyLength) {
// 	  let code = body.charCodeAt(position);
// 	  if (isNameContinue(code)) {
// 	    ++position;
// 	  } else {
// 	    break;
// 	  }
// 	}
// 	return createToken(
// 	  lexer,
// 	  TokenKind.NAME,
// 	  start,
// 	  position,
// 	  body.slice(start, position),
// 	);
//       }

fn is_punctuator_token_kind(kind: TokenKind) -> boolean {
	return match kind {
		TokenKind::BANG => true,
		TokenKind::DOLLAR => true,
		TokenKind::AMP => true,
		TokenKind::PAREN_L => true,
		TokenKind::PAREN_R => true,
		TokenKind::SPREAD => true,
		TokenKind::COLON => true,
		TokenKind::EQUALS => true,
		TokenKind::AT => true,
		TokenKind::BRACKET_L => true,
		TokenKind::BRACKET_R => true,
		TokenKind::BRACE_L => true,
		TokenKind::BRACE_R => true,
		TokenKind::PIPE => true,
		_ => false,
	};
}
