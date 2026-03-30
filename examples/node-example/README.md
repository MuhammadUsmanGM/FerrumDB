# Node.js Example — Note Taker

A small note-taking app demonstrating FerrumDB's Node.js API.

## Setup

First, build the native addon:

```bash
cd ../../ferrumdb-node
npm install
npm run build
```

## Run

```bash
node main.mjs
```

## What it shows

- Adding and reading JSON notes
- Secondary indexes (query notes by tag)
- Atomic transactions (bulk-add notes)
- Deleting entries
- Listing all keys
