import { defineConfig } from 'tsup';

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    'workflows/index': 'src/workflows/index.ts',
    'context/index': 'src/context/index.ts',
    'conversations/index': 'src/conversations/index.ts',
  },
  format: ['cjs', 'esm'],
  dts: true,
  sourcemap: true,
  clean: true,
  minify: false,
  splitting: false,
  treeshake: true,
  target: 'es2022',
  outDir: 'dist',
});
