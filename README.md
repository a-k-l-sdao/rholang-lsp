# rholang-lsp

A language server for [Rholang](https://rholang.io), the process calculus language used by F1R3FLY / RChain.

Built with tree-sitter for fast, incremental parsing. No running node required.

## Features

- **Diagnostics** — syntax errors highlighted as you type
- **Document symbols** — contracts and channel declarations in outline view
- **Goto definition** — jump to where a name is declared (`gd` in Neovim)
- **Find references** — all usages of a name in the current file (`gr` in Neovim)
- **Hover** — node type, context, and doc comments (`K` in Neovim)
- **Rename** — rename a symbol across all references
- **Semantic tokens** — rich syntax highlighting (keywords, functions, parameters, types, etc.)

## Install

### From source

```bash
git clone https://github.com/a-k-l-sdao/rholang-lsp
cd rholang-lsp
cargo build --release
cp target/release/rholang-lsp ~/.local/bin/
```

Requires the [tree-sitter-rholang](https://github.com/a-k-l-sdao/tree-sitter-rholang) grammar (pulled automatically via Cargo path dependency — clone it alongside this repo).

## Editor Setup

### Neovim (v0.11+)

Add to your `init.lua`:

```lua
vim.lsp.config("rholang-lsp", {
    cmd = { "rholang-lsp", "--stdio" },
    filetypes = { "rholang" },
    root_markers = { ".git", "rholang.toml" },
})
vim.lsp.enable("rholang-lsp")
```

For tree-sitter highlighting, also install [tree-sitter-rholang](https://github.com/a-k-l-sdao/tree-sitter-rholang).

### VSCode

Install the extension from the `editors/vscode/` directory:

```bash
cd editors/vscode
npm install
npm run compile
npx @vscode/vsce package --allow-missing-repository
code --install-extension rholang-lsp-0.1.0.vsix
```

Then set the binary path in VSCode settings if it's not on your PATH:

```json
{
    "rholang.lsp.path": "/path/to/rholang-lsp"
}
```

## CLI Flags

| Flag | Description |
|---|---|
| `--stdio` | Use stdio transport (required) |
| `--log-level <level>` | `trace`, `debug`, `info`, `warn`, `error` (default: `warn`) |
| `--no-color` | Disable color in log output |

## Architecture

```
src/
├── main.rs              # CLI + server startup
├── backend.rs           # LanguageServer trait implementation (tower-lsp)
├── document.rs          # Per-document state (source text + tree-sitter Tree)
├── diagnostics.rs       # ERROR/MISSING nodes → LSP diagnostics
├── symbols.rs           # documentSymbol (contracts, channels)
├── definition.rs        # goto definition + find references (scope-aware)
├── hover.rs             # node info + doc comments
├── rename.rs            # workspace-wide rename via references
└── semantic_tokens.rs   # AST walk → semantic token array
queries/
├── locals.scm           # scope/definition/reference queries
└── highlights.scm       # token classification
editors/
└── vscode/              # VSCode extension
```

## License

MIT
