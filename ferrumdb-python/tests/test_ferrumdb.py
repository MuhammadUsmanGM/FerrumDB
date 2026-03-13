"""Tests for FerrumDB Python bindings."""
import pytest
from ferrumdb import FerrumDB, Transaction


class TestBasicOperations:
    """Test basic CRUD operations."""

    def test_set_and_get(self, tmp_path):
        """Test setting and retrieving a value."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("key1", {"name": "alice", "score": 99})
        result = db.get("key1")
        
        assert result is not None
        # Result is JSON string for objects
        import json
        data = json.loads(result)
        assert data["name"] == "alice"
        assert data["score"] == 99

    def test_get_nonexistent_key(self, tmp_path):
        """Test getting a key that doesn't exist."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        result = db.get("nonexistent")
        assert result is None

    def test_delete(self, tmp_path):
        """Test deleting a key."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("key1", "value1")
        deleted = db.delete("key1")
        assert deleted is True
        
        # Verify deleted
        result = db.get("key1")
        assert result is None
        
        # Delete non-existent key
        deleted = db.delete("nonexistent")
        assert deleted is False

    def test_count_and_keys(self, tmp_path):
        """Test counting and listing keys."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        assert db.count() == 0
        
        db.set("user:1", {"name": "alice"})
        db.set("user:2", {"name": "bob"})
        db.set("counter", 42)
        
        assert db.count() == 3
        keys = db.keys()
        assert len(keys) == 3
        assert "user:1" in keys
        assert "user:2" in keys
        assert "counter" in keys


class TestSecondaryIndexing:
    """Test secondary index functionality."""

    def test_create_index_and_find(self, tmp_path):
        """Test creating an index and querying."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        # Add data
        db.set("user:1", {"name": "alice", "role": "admin"})
        db.set("user:2", {"name": "bob", "role": "user"})
        db.set("user:3", {"name": "charlie", "role": "admin"})
        
        # Create index
        db.create_index("role")
        
        # Query
        admins = db.find("role", '"admin"')
        assert len(admins) == 2
        assert "user:1" in admins
        assert "user:3" in admins
        
        users = db.find("role", '"user"')
        assert len(users) == 1
        assert "user:2" in users

    def test_index_updates_on_set(self, tmp_path):
        """Test that index updates when data changes."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("user:1", {"name": "alice", "role": "user"})
        db.create_index("role")
        
        # Initially alice is a user
        users = db.find("role", '"user"')
        assert "user:1" in users
        
        # Promote to admin
        db.set("user:1", {"name": "alice", "role": "admin"})
        
        admins = db.find("role", '"admin"')
        assert "user:1" in admins
        
        users = db.find("role", '"user"')
        assert "user:1" not in users


class TestTransactions:
    """Test transaction functionality."""

    def test_transaction_commit(self, tmp_path):
        """Test committing a transaction."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        tx = Transaction()
        tx.set("key1", {"value": 1})
        tx.set("key2", {"value": 2})
        tx.delete("key1")
        
        db.commit(tx)
        
        # key1 was set then deleted
        assert db.get("key1") is None
        # key2 was set
        assert db.get("key2") is not None

    def test_transaction_repr(self):
        """Test transaction string representation."""
        tx = Transaction()
        tx.set("key1", {"value": 1})
        tx.set("key2", {"value": 2})
        
        repr_str = repr(tx)
        assert "ops=2" in repr_str


class TestTypes:
    """Test different data types."""

    def test_string_value(self, tmp_path):
        """Test storing string values."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("greeting", "hello world")
        result = db.get("greeting")
        assert result == "hello world"

    def test_integer_value(self, tmp_path):
        """Test storing integer values."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("counter", 42)
        result = db.get("counter")
        assert result == 42

    def test_float_value(self, tmp_path):
        """Test storing float values."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("price", 19.99)
        result = db.get("price")
        assert abs(result - 19.99) < 0.01

    def test_boolean_value(self, tmp_path):
        """Test storing boolean values."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("active", True)
        result = db.get("active")
        assert result is True

    def test_list_value(self, tmp_path):
        """Test storing list values."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        db.set("tags", ["python", "rust", "database"])
        result = db.get("tags")
        assert isinstance(result, list)
        assert len(result) == 3


class TestDatabaseRepr:
    """Test database string representation."""

    def test_repr(self, tmp_path):
        """Test database __repr__."""
        db_path = tmp_path / "test.db"
        db = FerrumDB.open(str(db_path))
        
        repr_str = repr(db)
        assert "FerrumDB" in repr_str
        assert "entries=0" in repr_str
        
        db.set("key1", "value1")
        repr_str = repr(db)
        assert "entries=1" in repr_str
