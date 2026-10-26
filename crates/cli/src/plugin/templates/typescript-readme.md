# {{NAME}}

Node 20+ Agent Plugin scaffold using the stable 1.x MCP TypeScript SDK.
The tracked `skills/` directory is intentionally empty; add standard
`skills/<name>/SKILL.md` packages when the plugin needs reusable guidance.
The empty `io.github.okikorg.orcacode/hooks.json` is FineCore's optional
client extension in a retained compatibility namespace; Agent Plugins 1.0
itself standardizes only Skills and MCP.

FineCore does not install dependencies. Install, test, and build explicitly:

```sh
npm ci
npm test
npm run build
finecore plugin validate .
finecore plugin test .
```

`finecore plugin test .` fails until `dist/server.mjs` has been built.
