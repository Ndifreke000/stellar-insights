#!/usr/bin/env node

import fs from 'fs';
import path from 'path';

const BUNDLE_SIZE_LIMIT = 200 * 1024;
const CHUNK_SIZE_LIMIT = 100 * 1024;

function analyzeBundle() {
  const jsDir = path.join(process.cwd(), '.next/static/chunks');
  
  if (!fs.existsSync(jsDir)) {
    console.error('❌ Build directory not found. Run "npm run build" first.');
    process.exit(1);
  }

  const files = fs.readdirSync(jsDir).filter(f => f.endsWith('.js'));
  let totalSize = 0;
  let mainBundleSize = 0;
  const chunks = [];

  files.forEach(file => {
    const filePath = path.join(jsDir, file);
    const size = fs.statSync(filePath).size;
    totalSize += size;

    if (file.includes('main')) {
      mainBundleSize = size;
    }

    chunks.push({ file, size });
  });

  chunks.sort((a, b) => b.size - a.size);

  console.log('\n📊 Bundle Analysis Report\n');
  console.log(`Total JS Size: ${(totalSize / 1024).toFixed(2)} KB`);
  console.log(`Main Bundle: ${(mainBundleSize / 1024).toFixed(2)} KB`);
  
  console.log('\n📦 Top 10 Chunks:');
  chunks.slice(0, 10).forEach(({ file, size }) => {
    const sizeKB = (size / 1024).toFixed(2);
    const status = size > CHUNK_SIZE_LIMIT ? '⚠️' : '✅';
    console.log(`  ${status} ${file}: ${sizeKB} KB`);
  });

  console.log('\n✅ Verification Results:');
  
  const mainBundleOk = mainBundleSize < BUNDLE_SIZE_LIMIT;
  console.log(`  ${mainBundleOk ? '✅' : '❌'} Main bundle < 200KB: ${(mainBundleSize / 1024).toFixed(2)} KB`);
  
  const codeSplitOk = chunks.length > 1;
  console.log(`  ${codeSplitOk ? '✅' : '❌'} Code splitting enabled: ${chunks.length} chunks`);
  
  const largeChunks = chunks.filter(c => c.size > CHUNK_SIZE_LIMIT).length;
  console.log(`  ${largeChunks === 0 ? '✅' : '⚠️'} Chunks < 100KB: ${chunks.length - largeChunks}/${chunks.length}`);

  if (!mainBundleOk || !codeSplitOk) {
    console.log('\n❌ Bundle size targets not met!');
    process.exit(1);
  }

  console.log('\n✅ All bundle size targets met!\n');
}

analyzeBundle();
