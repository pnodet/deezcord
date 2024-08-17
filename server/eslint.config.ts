// eslint-disable-next-line node/no-unpublished-import
import { nivalis } from '@nivalis/eslint-config';

export default nivalis(
  {
    typescript: {
      configPath: './tsconfig.json',
    },
  },
  {
    rules: {
      '@typescript-eslint/no-magic-numbers': 'off',
      'node/no-unsupported-features/node-builtins': 'off',
    },
  },
);
