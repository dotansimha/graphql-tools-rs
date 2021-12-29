mod source;
use crate::source::Source;

/// Represents a location in a Source.
pub struct SourceLocation {
	line: usize,
	column: usize,
}

pub fn get_location(source: &Source, position: usize) -> SourceLocation {
	let mut line = 1;
	let mut column = 1;
	let mut index = 0;
	while index < position {
		if source.get_body().chars().nth(index).unwrap() == '\n' {
			line += 1;
			column = 1;
		} else {
			column += 1;
		}
		index += 1;
	}
	SourceLocation { line, column }
}
