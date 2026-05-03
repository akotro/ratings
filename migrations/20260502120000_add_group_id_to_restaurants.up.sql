-- Drop foreign keys that reference the old string-based restaurant id
ALTER TABLE ratings DROP FOREIGN KEY IF EXISTS ratings_ibfk_1;
ALTER TABLE rating_notifications DROP FOREIGN KEY IF EXISTS rating_notifications_ibfk_2;
ALTER TABLE restaurant_menu_items DROP FOREIGN KEY IF EXISTS restaurant_menu_items_ibfk_1;

-- Add group_id column
ALTER TABLE restaurants ADD COLUMN group_id CHAR(36) NULL;

-- For fresh DBs: create a default group so restaurants have something to reference
INSERT INTO groups (id, name, description)
SELECT '00000000-0000-0000-0000-000000000000', 'Default Group', 'Migration Default'
WHERE NOT EXISTS (SELECT 1 FROM groups LIMIT 1);

-- Assign only restaurants with NULL group_id to the first group (preserve existing group_id from fixtures)
UPDATE restaurants SET group_id = (SELECT id FROM groups LIMIT 1) WHERE group_id IS NULL;

-- Rename original id to preserve the name-based identifier
ALTER TABLE restaurants CHANGE COLUMN id restaurant_code VARCHAR(255) NOT NULL;

-- Replace string-based PK with auto-increment integer
ALTER TABLE restaurants DROP PRIMARY KEY;
ALTER TABLE restaurants ADD COLUMN id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY FIRST;

-- Prevent duplicate restaurants per group (unique constraint handles this)
ALTER TABLE restaurants ADD UNIQUE INDEX idx_restaurant_code_group (restaurant_code, group_id);

-- Give each group a copy of every restaurant
INSERT IGNORE INTO restaurants (restaurant_code, cuisine, group_id)
SELECT r.restaurant_code, r.cuisine, g.id
FROM (SELECT DISTINCT restaurant_code, cuisine FROM restaurants) r
CROSS JOIN groups g;

-- Remove temporary default group created above (keep real groups)
DELETE FROM groups WHERE id = '00000000-0000-0000-0000-000000000000'
AND (SELECT COUNT(*) FROM groups WHERE id != '00000000-0000-0000-0000-000000000000') > 0;

ALTER TABLE restaurants MODIFY COLUMN group_id CHAR(36) NOT NULL;

ALTER TABLE restaurants ADD CONSTRAINT fk_restaurants_group_id FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE;

-- Update ratings to reference restaurants in the same group
ALTER TABLE ratings ADD COLUMN restaurant_code_temp VARCHAR(255);
UPDATE ratings SET restaurant_code_temp = restaurant_id;

UPDATE ratings r
INNER JOIN restaurants newr ON newr.restaurant_code = r.restaurant_code_temp AND newr.group_id = r.group_id
SET r.restaurant_id = newr.id;

ALTER TABLE ratings DROP COLUMN restaurant_code_temp;

-- Same for rating_notifications
ALTER TABLE rating_notifications ADD COLUMN restaurant_code_temp VARCHAR(255);
UPDATE rating_notifications SET restaurant_code_temp = restaurant_id;

UPDATE rating_notifications r
INNER JOIN restaurants newr ON newr.restaurant_code = r.restaurant_code_temp AND newr.group_id = r.group_id
SET r.restaurant_id = newr.id;

ALTER TABLE rating_notifications DROP COLUMN restaurant_code_temp;

ALTER TABLE rating_notifications MODIFY COLUMN restaurant_id INT NOT NULL;
ALTER TABLE rating_notifications ADD CONSTRAINT fk_rating_notifications_restaurant_id FOREIGN KEY (restaurant_id) REFERENCES restaurants(id) ON DELETE CASCADE;

ALTER TABLE ratings MODIFY COLUMN restaurant_id INT NOT NULL;
ALTER TABLE ratings ADD CONSTRAINT fk_ratings_restaurant_id FOREIGN KEY (restaurant_id) REFERENCES restaurants(id) ON DELETE CASCADE;

DROP TABLE IF EXISTS restaurant_menu_items;