import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { FerrumDb: FerrumDB, Transaction } = require('./index.js');

// Open (or create) a database
const db = FerrumDB.open("example.db");

// Basic CRUD
db.set("user:1", { name: "alice", role: "admin" });
db.set("user:2", { name: "bob", role: "user" });
db.set("user:3", { name: "charlie", role: "admin" });
db.set("counter", 42);

console.log("Get user:1 =>", db.get("user:1"));
console.log("Get counter =>", db.get("counter"));
console.log("Total keys:", db.count());
console.log("All keys:", db.keys());

// Delete
const deleted = db.delete("counter");
console.log("Deleted counter:", deleted);
console.log("Get counter after delete =>", db.get("counter"));

// Secondary indexes
db.createIndex("role");
const admins = db.find("role", '"admin"');
console.log("Admins:", admins);

// Transactions
const tx = new Transaction();
tx.set("tx:1", { batch: true, seq: 1 });
tx.set("tx:2", { batch: true, seq: 2 });
tx.delete("user:3");
db.commit(tx);

console.log("After transaction:");
console.log("  tx:1 =>", db.get("tx:1"));
console.log("  tx:2 =>", db.get("tx:2"));
console.log("  user:3 =>", db.get("user:3"));
console.log("  Total keys:", db.count());

console.log("\nDone!");
