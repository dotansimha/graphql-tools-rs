## `graphql-tools` (Rust)

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
- [x] Visitor: `SchemaVisitor`
- [x] Visitor: `QueryVisitor`
- [x] GraphQL Validation engine
- [ ] Validation rules (in-progress)

> If you have an idea / missing feature, feel free to open an issue / start a GitHub discussion!
