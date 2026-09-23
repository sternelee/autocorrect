#!/usr/bin/env node
// Merge cspell plain-text dictionaries into a single word list for the
// Rust backend to include at compile time.

import { createReadStream } from "node:fs";
import { createWriteStream } from "node:fs";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { glob } from "node:fs/promises";
import { createGunzip } from "node:zlib";
import path from "node:path";
import readline from "node:readline";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const outPath = path.join(
  repoRoot,
  "src-tauri",
  "dictionaries",
  "cspell-merged.txt",
);

const cspellTrieBin = path.join(
  repoRoot,
  "node_modules",
  ".bin",
  "cspell-trie",
);

// Plain .txt dictionaries to include as-is.
const txtGlobs = [
  "node_modules/@cspell/dict-companies/dict/*.txt",
  "node_modules/@cspell/dict-software-terms/dict/*.txt",
];

// Gzipped plain .txt dictionaries.
const txtGzGlobs = ["node_modules/@cspell/dict-software-terms/dict/*.txt.gz"];

// CSpell binary trie dictionaries that can be dumped with cspell-trie.
const trieGlobs = ["node_modules/@cspell/dict-en_us/*.trie.gz"];

// Existing bundled dictionary (preserves project-specific additions).
const bundledPath = path.join(
  repoRoot,
  "src-tauri",
  "dictionaries",
  "cspell-bundled.txt",
);

async function* readLines(filePath) {
  const stream = createReadStream(filePath);
  const rl = readline.createInterface({ input: stream, crlfDelay: Infinity });
  for await (const line of rl) {
    yield line;
  }
}

async function* readGzLines(filePath) {
  const stream = createReadStream(filePath).pipe(createGunzip());
  const rl = readline.createInterface({ input: stream, crlfDelay: Infinity });
  for await (const line of rl) {
    yield line;
  }
}

async function* readTrieWords(filePath) {
  const child = spawn(cspellTrieBin, ["reader", filePath], {
    stdio: ["ignore", "pipe", "inherit"],
  });
  const rl = readline.createInterface({
    input: child.stdout,
    crlfDelay: Infinity,
  });
  for await (const line of rl) {
    yield line;
  }
}

async function* allWords() {
  // Existing bundled dictionary first.
  for await (const line of readLines(bundledPath)) {
    yield line;
  }

  for (const pattern of txtGlobs) {
    for await (const file of glob(pattern, { cwd: repoRoot })) {
      for await (const line of readLines(path.join(repoRoot, file))) {
        yield line;
      }
    }
  }

  for (const pattern of txtGzGlobs) {
    for await (const file of glob(pattern, { cwd: repoRoot })) {
      for await (const line of readGzLines(path.join(repoRoot, file))) {
        yield line;
      }
    }
  }

  for (const pattern of trieGlobs) {
    for await (const file of glob(pattern, { cwd: repoRoot })) {
      for await (const line of readTrieWords(path.join(repoRoot, file))) {
        yield line;
      }
    }
  }
}

async function main() {
  const words = new Set();

  for await (const raw of allWords()) {
    const line = raw.trim();
    if (line.length === 0 || line.startsWith("#")) {
      continue;
    }
    // Lowercase everything; the Rust allow-list checks case-insensitively.
    words.add(line.toLowerCase());
  }

  const sorted = Array.from(words).sort();

  const out = createWriteStream(outPath);
  out.write(
    "# Auto-generated merged dictionary for AutoCorrect spell checking\n",
  );
  out.write("# Sources: cspell bundled/companies/software-terms/en_us\n");
  out.write(`# Word count: ${sorted.length}\n`);
  for (const word of sorted) {
    out.write(`${word}\n`);
  }
  out.end();

  await new Promise((resolve, reject) => {
    out.on("finish", resolve);
    out.on("error", reject);
  });

  console.log(
    `Merged ${sorted.length} words into ${path.relative(repoRoot, outPath)}`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
