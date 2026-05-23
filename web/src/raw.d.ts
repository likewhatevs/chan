declare module "*.svelte?raw" {
  const content: string;
  export default content;
}

declare module "*.ts?raw" {
  const content: string;
  export default content;
}

/// `fullstack-a-79` slice 3: process-template files bundled
/// into the SPA via vite `?raw`. The orchestrator imports
/// `docs/templates/team-process/*.tpl?raw` at build time +
/// substitutes the team's vars on bootstrap.
declare module "*.tpl?raw" {
  const content: string;
  export default content;
}

declare module "*.md?raw" {
  const content: string;
  export default content;
}

// Minimal `node:fs` shim for tests that need to read on-disk files
// the `?raw` Vite import can't surface (notably `.css`, which the
// CSS plugin chain consumes before vitest sees the file). The full
// `@types/node` package isn't a dev dep; this declaration carries
// just the read helpers we actually call.
declare module "node:fs" {
  export function readFileSync(path: string, encoding: string): string;
}
