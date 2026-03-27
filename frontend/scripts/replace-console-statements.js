#!/usr/bin/env node

/**
 * Script to replace console statements with logger calls
 * 
 * Usage: node scripts/replace-console-statements.js
 */

const fs = require('fs');
const path = require('path');
const { glob } = require('glob');

// Files to process
const patterns = [
  'src/**/*.ts',
  'src/**/*.tsx',
  '!src/**/*.test.ts',
  '!src/**/*.test.tsx',
  '!src/**/*.spec.ts',
  '!src/**/*.spec.tsx',
];

// Replacement mappings
const replacements = [
  {
    // console.log -> logger.debug
    pattern: /console\.log\(/g,
    replacement: 'logger.debug(',
  },
  {
    // console.info -> logger.info
    pattern: /console\.info\(/g,
    replacement: 'logger.info(',
  },
  {
    // console.warn -> logger.warn
    pattern: /console\.warn\(/g,
    replacement: 'logger.warn(',
  },
  {
    // console.error -> logger.error
    pattern: /console\.error\(/g,
    replacement: 'logger.error(',
  },
  {
    // console.debug -> logger.debug
    pattern: /console\.debug\(/g,
    replacement: 'logger.debug(',
  },
];

async function processFile(filePath) {
  try {
    let content = fs.readFileSync(filePath, 'utf8');
    let modified = false;
    let hasConsole = false;

    // Check if file has console statements
    for (const { pattern } of replacements) {
      if (pattern.test(content)) {
        hasConsole = true;
        break;
      }
    }

    if (!hasConsole) {
      return { processed: false, modified: false };
    }

    // Apply replacements
    for (const { pattern, replacement } of replacements) {
      if (pattern.test(content)) {
        content = content.replace(pattern, replacement);
        modified = true;
      }
    }

    if (!modified) {
      return { processed: true, modified: false };
    }

    // Check if logger import already exists
    const hasLoggerImport = /import\s+{[^}]*logger[^}]*}\s+from\s+['"]@\/lib\/logger['"]/.test(content) ||
                           /import\s+{\s*logger\s*}\s+from\s+['"]@\/lib\/logger['"]/.test(content);

    if (!hasLoggerImport) {
      // Find the last import statement
      const importRegex = /^import\s+.*?from\s+['"].*?['"];?\s*$/gm;
      const imports = content.match(importRegex);
      
      if (imports && imports.length > 0) {
        const lastImport = imports[imports.length - 1];
        const lastImportIndex = content.lastIndexOf(lastImport);
        const insertPosition = lastImportIndex + lastImport.length;
        
        // Insert logger import after last import
        content = content.slice(0, insertPosition) + 
                 "\nimport { logger } from '@/lib/logger';" +
                 content.slice(insertPosition);
      } else {
        // No imports found, add at the beginning
        content = "import { logger } from '@/lib/logger';\n\n" + content;
      }
    }

    // Write back to file
    fs.writeFileSync(filePath, content, 'utf8');
    
    return { processed: true, modified: true };
  } catch (error) {
    console.error(`Error processing ${filePath}:`, error.message);
    return { processed: false, modified: false, error: error.message };
  }
}

async function main() {
  console.log('🔍 Finding files with console statements...\n');

  const files = await glob(patterns, {
    cwd: path.join(__dirname, '..'),
    absolute: true,
    ignore: ['node_modules/**', '.next/**', 'dist/**', 'build/**'],
  });

  console.log(`Found ${files.length} files to check\n`);

  let processedCount = 0;
  let modifiedCount = 0;
  let errorCount = 0;

  for (const file of files) {
    const result = await processFile(file);
    
    if (result.processed) {
      processedCount++;
      
      if (result.modified) {
        modifiedCount++;
        const relativePath = path.relative(process.cwd(), file);
        console.log(`✅ Modified: ${relativePath}`);
      }
    }
    
    if (result.error) {
      errorCount++;
      const relativePath = path.relative(process.cwd(), file);
      console.log(`❌ Error: ${relativePath} - ${result.error}`);
    }
  }

  console.log('\n📊 Summary:');
  console.log(`   Total files checked: ${files.length}`);
  console.log(`   Files with console statements: ${processedCount}`);
  console.log(`   Files modified: ${modifiedCount}`);
  console.log(`   Errors: ${errorCount}`);
  
  if (modifiedCount > 0) {
    console.log('\n✨ Console statements have been replaced with logger calls!');
    console.log('   Please review the changes and run tests.');
  } else {
    console.log('\n✅ No console statements found or all already using logger!');
  }
}

main().catch(console.error);
