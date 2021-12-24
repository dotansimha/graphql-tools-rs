struct Location {
	line: usize,
	column: usize,
}

/// A representation of source input to GraphQL. The `name` and `locationOffset` parameters are
/// optional, but they are useful for clients who store GraphQL documents in source files.
/// For example, if the GraphQL input starts at line 40 in a file named `Foo.graphql`, it might
/// be useful for `name` to be `"Foo.graphql"` and location to be `{ line: 40, column: 1 }`.
/// The `line` and `column` properties in `locationOffset` are 1-indexed.
pub struct Source {
	name: String,
	body: String,
	location: Location,
}

impl Source {
	fn new(name: String, body: String, locationOffset: Option<Location>) -> Source {
		if locationOffset == None {
			Source {
				name,
				body,
				location: Location { line: 1, column: 1 },
			}
		} else {
			Source {
				name,
				body,
				location: locationOffset,
			}
		}
	}

	fn get_name(&self) -> &String {
		&self.name
	}

	fn get_body(&self) -> &String {
		&self.body
	}

	fn get_location(&self) -> &Location {
		&self.location
	}

	fn set_location(&mut self, line: usize, column: usize) {
		self.location.line = line;
		self.location.column = column;
	}
}
