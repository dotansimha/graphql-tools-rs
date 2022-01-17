## `graphql-tools` (Rust)

[Documentation](https://docs.rs/graphql-tools) | [Crate](https://crates.io/crates/graphql-tools) | [GitHub](https://github.com/dotansimha/graphql-tools-rs)

> **Note: this crate is still under development (see roadmap below)**

The [`graphql_tools` crate](https://crates.io/crates/graphql-tools) implements tooling around GraphQL for Rust libraries. Most of the tools are based on `trait`s and `struct`s implemented in [`graphql_parser` crate](https://crates.io/crates/graphql-parser).

The goal of this library is to create a common layer of tools that has similar/improved APIs to [`graphql-js` reference implementation](https://github.com/graphql/graphql-js) and [`graphql-tools` from the JS/TS ecosystem](https://github.com/ardatan/graphql-tools).

### Getting Started

![Crates.io](https://img.shields.io/crates/v/graphql-tools?label=graphql-tools%20%28crates.io%29)

Add `graphql-tools` as a dependency of your project by adding the following to your `Cargo.toml` file:

```toml
[dependencies]
graphql-tools = "..."
```

Or, if you are using [`cargo-edit`](https://github.com/killercup/cargo-edit):

```
cargo add graphql-tools
```

### Roadmap and progress

- [ ] Better documentation 
- [x] AST Visitor for GraphQL schema (`graphql_parser::schema::Document`)
- [x] AST Visitor for GraphQL operations (`graphql_parser::operation::Document`) 
- [x] AST Visitor with TypeInfo
- [x] AST tools (ongoing)
- [x] `struct` extensions
- [x] GraphQL Validation engine
- [ ] Validation rules (in-progress)

> If you have an idea / missing feature, feel free to open an issue / start a GitHub discussion!

#### Validation Rules

> This comparison is based on `graphql-js` refernece implementation. 

- [x] ExecutableDefinitions (not actually needed)
- [x] UniqueOperationNames
- [x] LoneAnonymousOperation
- [x] SingleFieldSubscriptions
- [x] KnownTypeNames
- [x] FragmentsOnCompositeTypes
- [x] VariablesAreInputTypes
- [x] LeafFieldSelections
- [x] FieldsOnCorrectType
- [x] UniqueFragmentNames
- [x] KnownFragmentNames
- [x] NoUnusedFragments
- [x] PossibleFragmentSpreads
- [x] NoFragmentCycles
- [x] UniqueVariableNames
- [x] NoUndefinedVariables
- [x] NoUnusedVariables
- [x] KnownDirectives
- [ ] UniqueDirectivesPerLocation
- [x] KnownArgumentNames
- [x] UniqueArgumentNames
- [ ] ValuesOfCorrectType
- [x] ProvidedRequiredArguments
- [ ] VariablesInAllowedPosition
- [x] OverlappingFieldsCanBeMerged
- [ ] UniqueInputFieldNames (blocked by https://github.com/graphql-rust/graphql-parser/issues/59)