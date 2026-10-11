# @almadar/mobile-shell

React Native mobile client template plus a verbatim TypeScript server template for the Almadar compiler.

## What it is

- `packages/mobile/` — React Native client template. The compiler generates screens, hooks, and a navigator into `packages/mobile/src/generated/`.
- `packages/server/` — TypeScript server template copied verbatim from `@almadar/shell-server` / `almadar-shell`. The mobile shell does **not** generate server code from the `.orb` schema; it reuses the static backend template.
- `packages/shared/` — Shared type definitions, including `entities.ts` generated from the `.orb` schema.

## Compile

Single output with the static TypeScript server template:

```bash
orb compile app.orb --shell mobile -o ./output
```

Separate frontend and backend outputs:

```bash
orb compile app.orb --frontend mobile --backend typescript -o ./output
```

## Generated output

```
./output/
├── packages/
│   ├── mobile/src/generated/
│   │   ├── screens/*.tsx              # One screen per orbital page
│   │   ├── hooks/use*.ts              # Generated React hooks
│   │   └── GeneratedNavigator.tsx     # Stack/tab navigator wiring
│   ├── shared/src/types/entities.ts   # Entity types from the schema
│   └── server/                        # Verbatim TypeScript server template
├── component-mapping.json
└── shell.toml
```

## Run the generated app

```bash
cd output
pnpm install
pnpm -F @almadar/mobile-shell-client typecheck
pnpm -F @almadar/shell-server typecheck
pnpm -F @almadar/mobile-shell-client start
```

## Notes

- The server is the TypeScript backend verbatim; the mobile backend does not generate server code.
- The generated client currently produces static screen scaffolding and type definitions. Full effect lowering and a runtime interpreter for mobile are not implemented yet.
- React 19 / React Native 0.76 peer-dependency warnings may affect runtime; track the upstream React Native upgrade.

## License

BSL 1.1 (Business Source License). Converts to Apache 2.0 on 2030-02-01. Non-production use is free.
