"""
FerrumDB Python Example
Run: python example.py   (after: maturin develop)
"""
from ferrumdb import FerrumDB

print("=== FerrumDB Python Bindings ===\n")

# 1. Open database
db = FerrumDB.open("example.db")
print(f"Opened: {db}")

# 2. Set values
db.set("user:1", '{"name": "alice", "role": "admin", "score": 99}')
db.set("user:2", '{"name": "bob", "role": "user", "score": 45}')
db.set("user:3", '{"name": "charlie", "role": "admin", "score": 77}')
db.set("counter", "42")
print(f"\nCount: {db.count()} entries")

# 3. Get a value
val = db.get("user:1")
print(f"\nGET user:1 => {val}")

# 4. List keys
print(f"\nKeys: {db.keys()}")

# 5. Delete
deleted = db.delete("counter")
print(f"\nDeleted 'counter': {deleted}")
print(f"Count after delete: {db.count()}")

# 6. Secondary indexing
db.create_index("role")
admins = db.find("role", '"admin"')
print(f"\nAdmins (via index): {admins}")

# 7. None for missing key
missing = db.get("doesnt_exist")
print(f"\nMissing key: {missing}")

print("\n✅ All operations successful!")
