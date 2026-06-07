# Knot

Knot aims to be a fast, extensible static-analysis platform capable of
enforcing organization-wide policies across polyglot monorepos. It uses
tree-sitter for the parsing layer, and rules are implemented in WASM.

The design deliberately avoids building a universal Abstract Syntax Tree (AST)
or Intermediate Representation (IR). Rules do not traverse raw ASTs across the
Wasm boundary. Instead, they consume Shared Semantic Facts (e.g., Binding,
Call, Import) that are computed by the host engine in a controlled manner.

**Architcture**

Source Files -> Tree-sitter Parsers -> Language Adapters -> Semantic Facts -> Wasm Rules -> Diagnostics + Fixes

## Rules

- Do red/green TDD
- Use conventional commits
- Keep commit messages short
