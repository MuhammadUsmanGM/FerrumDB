"""
Contact Book — A small FerrumDB example in Python.

Demonstrates: CRUD, secondary indexes, transactions, and key listing.

Install: pip install ferrumdb
Run:     python main.py
"""

from ferrumdb import FerrumDB, Transaction
import os

db = FerrumDB.open("contacts.db")

print("=== FerrumDB Contact Book ===\n")

# ── Add contacts ──────────────────────────────────────────────
db.set("contact:1", '{"name": "Alice", "email": "alice@example.com", "role": "engineer"}')
db.set("contact:2", '{"name": "Bob", "email": "bob@example.com", "role": "designer"}')
db.set("contact:3", '{"name": "Charlie", "email": "charlie@example.com", "role": "engineer"}')
db.set("contact:4", '{"name": "Diana", "email": "diana@example.com", "role": "manager"}')

print(f"Added {db.count()} contacts\n")

# ── Read a contact ────────────────────────────────────────────
val = db.get("contact:1")
print(f"Contact 1: {val}\n")

# ── Secondary indexes: query by role ──────────────────────────
db.create_index("role")

engineers = db.find("role", '"engineer"')
print(f"Engineers: {engineers}")

designers = db.find("role", '"designer"')
print(f"Designers: {designers}")

managers = db.find("role", '"manager"')
print(f"Managers:  {managers}\n")

# ── Transactions: bulk-add contacts atomically ────────────────
tx = Transaction()
tx.set("contact:5", '{"name": "Eve", "email": "eve@example.com", "role": "engineer"}')
tx.set("contact:6", '{"name": "Frank", "email": "frank@example.com", "role": "designer"}')
db.commit(tx)

print(f"After transaction: {db.count()} contacts")
engineers_after = db.find("role", '"engineer"')
print(f"Engineers now: {engineers_after}\n")

# ── Delete a contact ──────────────────────────────────────────
deleted = db.delete("contact:2")
print(f"Deleted Bob: {deleted}")
print(f"Bob after delete: {db.get('contact:2')}")
print(f"Final count: {db.count()}\n")

# ── List all keys ─────────────────────────────────────────────
keys = sorted(db.keys())
print(f"All keys: {keys}")

# Cleanup
os.remove("contacts.db")
