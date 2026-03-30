/**
 * Note Taker — A small FerrumDB example in Node.js.
 *
 * Demonstrates: CRUD, secondary indexes, transactions, and key listing.
 *
 * Setup: cd ../../ferrumdb-node && npm install && npm run build
 * Run:   node main.mjs
 */

import { createRequire } from "module";
import fs from "fs";
const require = createRequire(import.meta.url);
const { FerrumDb: FerrumDB, Transaction } = require("../../ferrumdb-node/index.js");

const db = FerrumDB.open("notes.db");

console.log("=== FerrumDB Note Taker ===\n");

// ── Add notes ──────────────────────────────────────────────────
db.set("note:1", { title: "Meeting notes", content: "Discussed Q3 roadmap", tag: "work" });
db.set("note:2", { title: "Grocery list", content: "Milk, eggs, bread", tag: "personal" });
db.set("note:3", { title: "Bug report", content: "Login fails on Safari", tag: "work" });
db.set("note:4", { title: "Book recommendation", content: "Read Designing Data-Intensive Applications", tag: "personal" });

console.log(`Added ${db.count()} notes\n`);

// ── Read a note ────────────────────────────────────────────────
const note = db.get("note:1");
console.log("Note 1:", note, "\n");

// ── Secondary indexes: query by tag ────────────────────────────
db.createIndex("tag");

const workNotes = db.find("tag", '"work"');
console.log("Work notes:", workNotes);

const personalNotes = db.find("tag", '"personal"');
console.log("Personal notes:", personalNotes, "\n");

// ── Transactions: bulk-add notes atomically ────────────────────
const tx = new Transaction();
tx.set("note:5", { title: "Sprint retro", content: "What went well, what to improve", tag: "work" });
tx.set("note:6", { title: "Recipe", content: "Pasta carbonara", tag: "personal" });
db.commit(tx);

console.log(`After transaction: ${db.count()} notes`);
const workAfter = db.find("tag", '"work"');
console.log("Work notes now:", workAfter, "\n");

// ── Delete a note ──────────────────────────────────────────────
const deleted = db.delete("note:2");
console.log(`Deleted grocery list: ${deleted}`);
console.log("Note 2 after delete:", db.get("note:2"));
console.log(`Final count: ${db.count()}\n`);

// ── List all keys ──────────────────────────────────────────────
const keys = db.keys().sort();
console.log("All keys:", keys);

// Cleanup
fs.unlinkSync("notes.db");
