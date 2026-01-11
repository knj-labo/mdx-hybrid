import { parseBlocks } from './index.js';

const input = "Text with <script>alert('xss')</script> and & symbols.";
const result = parseBlocks(input, { enableDirectives: false });
console.log('Blocks count:', result.blocks.length);
console.log('Blocks:', JSON.stringify(result.blocks, null, 2));
