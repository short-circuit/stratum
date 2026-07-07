/// <reference types="vite/client" />

declare const __APP_VERSION__: string;

declare module '*.svg?raw' {
  const content: string;
  export default content;
}

// d3-force-3d ships with its own types but Vite's worker bundling
// strips module resolution — this shim satisfies the TS compiler.
declare module 'd3-force-3d';
