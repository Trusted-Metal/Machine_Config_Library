import { cpSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
cpSync(join(__dirname, '../../schema'), join(__dirname, '../schema'), { recursive: true });
