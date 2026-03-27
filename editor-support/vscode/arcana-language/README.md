# Arcana VS Code Support

This package gives VS Code basic Arcana support:

- `.arc` file recognition
- line comments with `//`
- indentation after `:`
- TextMate syntax highlighting for the current Arcana surface

It is intentionally syntax-only. It does not provide semantic analysis, completions, go-to-definition, or diagnostics. Those would require a dedicated language server or semantic token provider.

## Install From Repo

From the repo root, the easiest path is the repo-backed installer:

```powershell
.\scripts\dev\install-vscode-arcana-language.ps1
```

That creates a junction inside your VS Code extensions directory pointing at this repo checkout:

- no VSIX packaging step
- updates come directly from repo edits
- reload VS Code after grammar/package changes

For VS Code Insiders:

```powershell
.\scripts\dev\install-vscode-arcana-language.ps1 -Flavor insiders
```

To remove the repo-backed install:

```powershell
.\scripts\dev\install-vscode-arcana-language.ps1 -Uninstall
```

## Local Dev Host

1. Open [package.json](/C:/Users/iseka/Documents/GitHub/Arcana/editor-support/vscode/arcana-language/package.json) in VS Code.
2. Press `F5` to launch an Extension Development Host and test `.arc` highlighting immediately.

## Packaging

If you want an installable VSIX:

```powershell
cd editor-support/vscode/arcana-language
npx @vscode/vsce package
code --install-extension .\arcana-language-0.0.1.vsix
```

## Next Steps

The obvious follow-up is an Arcana language server or semantic token provider. TextMate grammar is enough to make the language readable, but not enough to match the editor experience of `rust-analyzer`.
