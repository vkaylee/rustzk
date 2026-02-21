#!/usr/bin/env node

/**
 * ==============================================================================
 * ai-context-os Node.js Installer
 * Description: Integrates ai-context-os using the Pointer Architecture.
 * ==============================================================================
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Utility colors for console output
const colors = {
    reset: '\x1b[0m',
    green: '\x1b[32m',
    blue: '\x1b[34m',
    yellow: '\x1b[33m',
    red: '\x1b[31m',
    cyan: '\x1b[36m'
};

const __filename = fileURLToPath(import.meta.url);
const SOURCE_DIR = path.resolve(path.dirname(__filename), '..');

console.log(`${colors.blue}==============================${colors.reset}`);
console.log(`${colors.blue}  ai-context-os Installer   ${colors.reset}`);
console.log(`${colors.blue}==============================${colors.reset}`);

// Parse arguments
const args = process.argv.slice(2);
const helpParam = args.find(arg => arg === '--help' || arg === '-h');

if (helpParam || args.length === 0) {
    console.log(`${colors.yellow}Usage: npx ai-context-os <target_project_directory>${colors.reset}`);
    console.log('Example: npx ai-context-os ../my-existing-app');
    console.log('Or use "." for the current directory: npx ai-context-os .\n');
    process.exit(args.length === 0 ? 1 : 0);
}

const targetInput = args[0];
const TARGET_DIR = path.resolve(process.cwd(), targetInput);

if (!fs.existsSync(TARGET_DIR)) {
    console.error(`${colors.red}Error: Target directory '${TARGET_DIR}' does not exist.${colors.reset}`);
    console.error('Please provide a valid path to an existing project.');
    process.exit(1);
}

const OS_DIR = path.join(TARGET_DIR, '.ai-context-os');

console.log(`\n${colors.cyan}Target Project:${colors.reset} ${TARGET_DIR}`);
console.log(`${colors.cyan}Creating OS Directory:${colors.reset} ${OS_DIR}`);

// Create `.ai-context-os/` in target
fs.mkdirSync(OS_DIR, { recursive: true });

console.log(`\n${colors.yellow}Copying core OS files...${colors.reset}`);

const FILES_TO_COPY = [
    'PROJECT_OS.md',
    '.cursorrules',
    'CLAUDE.md',
    'GEMINI.md'
];

const DIRS_TO_COPY = [
    'skills'
];

// Copy files
for (const file of FILES_TO_COPY) {
    const srcPath = path.join(SOURCE_DIR, file);
    const destPath = path.join(OS_DIR, file);

    if (fs.existsSync(srcPath)) {
        console.log(`  Copying ${file}...`);
        fs.copyFileSync(srcPath, destPath);
    } else {
        console.warn(`${colors.yellow}Warning: Source file '${file}' not found.${colors.reset}`);
    }
}

// Helper to copy directory recursively
function copyDirSync(src, dest) {
    fs.mkdirSync(dest, { recursive: true });
    let entries = fs.readdirSync(src, { withFileTypes: true });

    for (let entry of entries) {
        let srcPath = path.join(src, entry.name);
        let destPath = path.join(dest, entry.name);

        entry.isDirectory() ? copyDirSync(srcPath, destPath) : fs.copyFileSync(srcPath, destPath);
    }
}

// Copy directories
for (const dir of DIRS_TO_COPY) {
    const srcPath = path.join(SOURCE_DIR, dir);
    const destPath = path.join(OS_DIR, dir);

    if (fs.existsSync(srcPath)) {
        console.log(`  Copying directory ${dir}/...`);
        copyDirSync(srcPath, destPath);
    } else {
        console.warn(`${colors.yellow}Warning: Source directory '${dir}' not found.${colors.reset}`);
    }
}

console.log(`\n${colors.yellow}Creating Pointer Files in root...${colors.reset}`);

const cursorrulesContent = `# Pointer: .cursorrules
Read and adhere to the entire AI Operating System at: \`.ai-context-os/PROJECT_OS.md\` and skills at \`.ai-context-os/skills/\`.
If the project has a custom directory like \`.local-os/\`, prioritize using \`.local-os/\` first (Fallback Architecture).
Default AI adapter reference: \`.ai-context-os/.cursorrules\`
`;

const claudeContent = `# Pointer: CLAUDE.md
Read and adhere to the entire AI Operating System at: \`.ai-context-os/PROJECT_OS.md\` and skills at \`.ai-context-os/skills/\`.
If the project has a custom directory like \`.local-os/\`, prioritize using \`.local-os/\` first (Fallback Architecture).
Default AI adapter reference: \`.ai-context-os/CLAUDE.md\`
`;

const geminiContent = `# Pointer: GEMINI.md
Read and adhere to the entire AI Operating System at: \`.ai-context-os/PROJECT_OS.md\` and skills at \`.ai-context-os/skills/\`.
If the project has a custom directory like \`.local-os/\`, prioritize using \`.local-os/\` first (Fallback Architecture).
Default AI adapter reference: \`.ai-context-os/GEMINI.md\`
`;

let isSelfInstall = false;
try {
    const targetPkgPath = path.join(TARGET_DIR, 'package.json');
    if (fs.existsSync(targetPkgPath)) {
        const targetPkg = JSON.parse(fs.readFileSync(targetPkgPath, 'utf8'));
        if (targetPkg.name === 'ai-context-os') {
            isSelfInstall = true;
        }
    }
} catch (e) {
    // Ignore errors
}

if (isSelfInstall) {
    console.log(`  ${colors.cyan}[Dogfooding Mode] Target is 'ai-context-os' source repo.${colors.reset}`);
    console.log(`  Skipping pointer file generation to protect source L1 Adapters.`);
} else {
    fs.writeFileSync(path.join(TARGET_DIR, '.cursorrules'), cursorrulesContent, 'utf8');
    console.log('  Created .cursorrules pointer.');

    fs.writeFileSync(path.join(TARGET_DIR, 'CLAUDE.md'), claudeContent, 'utf8');
    console.log('  Created CLAUDE.md pointer.');

    fs.writeFileSync(path.join(TARGET_DIR, 'GEMINI.md'), geminiContent, 'utf8');
    console.log('  Created GEMINI.md pointer.');
}

console.log(`\n${colors.green}✅ Integration Complete!${colors.reset}`);
console.log(`The AI Context OS has been installed in: ${OS_DIR}`);
console.log(`\n${colors.yellow}Next Steps:${colors.reset}`);
console.log(`1. Navigate to your project: cd ${TARGET_DIR}`);
console.log(`2. The AI will now automatically read from .ai-context-os/ via the pointer files.`);
console.log(`3. To override rules, create your own local skills (e.g., in .local-os/skills/) rather than modifying the core OS files.`);
console.log(`4. Reload your AI assistant (Cursor/Claude/Gemini) window.\n`);
