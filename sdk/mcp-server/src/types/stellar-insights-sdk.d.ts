// @stellar-insights/sdk's own `tsup --dts` build currently fails (typescript
// 7.0.2 is incompatible with tsup's bundled rollup-plugin-dts, and `tsc
// --emitDeclarationOnly` surfaces further pre-existing type errors inside the
// SDK itself), so no .d.ts ships from that package yet. This shim lets the
// MCP server consume its runtime exports until the SDK's own build is fixed.
declare module "@stellar-insights/sdk";
