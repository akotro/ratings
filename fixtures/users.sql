INSERT INTO users (id, username, password) VALUES
    ('test_id', 'test_username', 'test_password'),
    ('test_id2', 'test_username2', 'test_password2'),
    ('test_id3', 'test_username3', 'test_password3');

INSERT INTO groups (id, name, description) VALUES
    ("test_group_id1", "test_group1", "this is test group 1 (users: test_id, test_id2)"),
    ("test_group_id2", "test_group2", "this is test group 2 (users: test_id, test_id3)");

INSERT INTO group_memberships (group_id, user_id, role) VALUES
    ("test_group_id1", "test_id", "admin"),
    ("test_group_id1", "test_id2", "member"),
    ("test_group_id2", "test_id", "admin");
