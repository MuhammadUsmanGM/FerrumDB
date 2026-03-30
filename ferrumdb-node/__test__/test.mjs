import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'module';
import fs from 'fs';

const require = createRequire(import.meta.url);
const { FerrumDb: FerrumDB, Transaction } = require('../index.js');

const TEST_DB = 'test_ferrumdb_node.db';

function cleanup() {
    try { fs.unlinkSync(TEST_DB); } catch {}
}

describe('FerrumDB Node.js Bindings', () => {
    before(cleanup);
    after(cleanup);

    it('should open a database', () => {
        const db = FerrumDB.open(TEST_DB);
        assert.ok(db);
    });

    it('should set and get values', () => {
        cleanup();
        const db = FerrumDB.open(TEST_DB);

        db.set("key1", "hello");
        db.set("key2", 42);
        db.set("key3", { name: "alice", role: "admin" });

        assert.equal(db.get("key1"), "hello");
        assert.equal(db.get("key2"), 42);
        assert.deepEqual(db.get("key3"), { name: "alice", role: "admin" });
    });

    it('should return null for missing keys', () => {
        cleanup();
        const db = FerrumDB.open(TEST_DB);
        assert.equal(db.get("nonexistent"), null);
    });

    it('should delete keys', () => {
        cleanup();
        const db = FerrumDB.open(TEST_DB);

        db.set("to_delete", "value");
        assert.equal(db.delete("to_delete"), true);
        assert.equal(db.get("to_delete"), null);
        assert.equal(db.delete("to_delete"), false);
    });

    it('should list keys and count', () => {
        cleanup();
        const db = FerrumDB.open(TEST_DB);

        db.set("a", 1);
        db.set("b", 2);
        db.set("c", 3);

        assert.equal(db.count(), 3);
        const keys = db.keys().sort();
        assert.deepEqual(keys, ["a", "b", "c"]);
    });

    it('should support secondary indexes', () => {
        cleanup();
        const db = FerrumDB.open(TEST_DB);

        db.set("u1", { name: "alice", role: "admin" });
        db.set("u2", { name: "bob", role: "user" });
        db.set("u3", { name: "charlie", role: "admin" });

        db.createIndex("role");

        const admins = db.find("role", '"admin"').sort();
        assert.deepEqual(admins, ["u1", "u3"]);

        const users = db.find("role", '"user"');
        assert.deepEqual(users, ["u2"]);
    });

    it('should support transactions', () => {
        cleanup();
        const db = FerrumDB.open(TEST_DB);

        db.set("existing", "will be deleted");

        const tx = new Transaction();
        tx.set("tx1", { val: 1 });
        tx.set("tx2", { val: 2 });
        tx.delete("existing");
        db.commit(tx);

        assert.deepEqual(db.get("tx1"), { val: 1 });
        assert.deepEqual(db.get("tx2"), { val: 2 });
        assert.equal(db.get("existing"), null);
        assert.equal(db.count(), 2);
    });

    it('should recover data after reopen', () => {
        cleanup();
        const path = 'test_recovery_node.db';
        try { fs.unlinkSync(path); } catch {}

        // Write
        const db1 = FerrumDB.open(path);
        db1.set("persist", { saved: true });

        // Reopen and verify
        const db2 = FerrumDB.open(path);
        assert.deepEqual(db2.get("persist"), { saved: true });

        try { fs.unlinkSync(path); } catch {}
    });
});
