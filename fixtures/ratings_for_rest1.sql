-- INSERT INTO ratings (group_id, restaurant_id, user_id, username, score, created_at, updated_at) VALUES
--      ('test_group_id1', 100, 'test_id', 'test_username', 4.5, NOW(), NOW()),
--      ('test_group_id1', 100, 'test_id2', 'test_username2', 3.0, NOW(), NOW());
INSERT INTO ratings (group_id, restaurant_id, user_id, username, score, created_at, updated_at)
SELECT 'test_group_id1', id, 'test_id', 'test_username', 4.5, NOW(), NOW()
FROM restaurants WHERE group_id = 'test_group_id1' AND restaurant_code = 'ARMYRA BY PAPAIOANNOU'
UNION ALL
SELECT 'test_group_id1', id, 'test_id2', 'test_username2', 3.0, NOW(), NOW()
FROM restaurants WHERE group_id = 'test_group_id1' AND restaurant_code = 'ARMYRA BY PAPAIOANNOU';
