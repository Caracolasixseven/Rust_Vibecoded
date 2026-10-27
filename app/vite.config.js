import { defineConfig } from 'vite';
import { cpSync } from 'node:fs';

function preserveDirectServeVendor() {
  return {
    name: 'preserve-direct-serve-vendor',
    closeBundle() { cpSync('www/vendor', 'dist/vendor', { recursive: true }); },
  };
}

// Relative base supports GitHub Pages project subpaths.
export default defineConfig({
  root: 'www',
  base: './',
  publicDir: false,
  server: { host: '127.0.0.1' },
  build: { target: 'es2022', outDir: '../dist', emptyOutDir: true },
  plugins: [preserveDirectServeVendor()],
});
