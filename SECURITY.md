# Security

## Reporting a vulnerability

Please report security issues responsibly. You can open a private issue or contact the maintainers (e.g. via the repository owner) so we can address it before public disclosure.

## Design notes

- **No Lua execution:** The parser only reads Lua-like data (tables, strings, numbers, booleans). It never executes Lua code.
- **Bounded input:** Blueprint file count and file size are limited to reduce risk of resource exhaustion.
- **Local-only:** The tool does not contact the network by default. All data is read from paths you provide.
