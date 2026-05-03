-- Recreate restaurant_menu_items table (without FK constraints, will be added by original migration)
CREATE TABLE restaurant_menu_items (
    id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
    restaurant_id VARCHAR(255) NOT NULL,
    menu_item_id INTEGER NOT NULL
);

-- Drop foreign key constraints first
ALTER TABLE ratings DROP FOREIGN KEY fk_ratings_restaurant_id;
ALTER TABLE rating_notifications DROP FOREIGN KEY fk_rating_notifications_restaurant_id;

-- Change ratings.restaurant_id from INTEGER to VARCHAR first
ALTER TABLE ratings MODIFY COLUMN restaurant_id VARCHAR(255);

-- Restore ratings to use name-based restaurant identifiers
ALTER TABLE ratings ADD COLUMN restaurant_id_temp VARCHAR(255);
UPDATE ratings r
INNER JOIN restaurants newr ON r.restaurant_id = newr.id
SET r.restaurant_id_temp = newr.restaurant_code;
UPDATE ratings SET restaurant_id = restaurant_id_temp WHERE restaurant_id_temp IS NOT NULL;
ALTER TABLE ratings DROP COLUMN restaurant_id_temp;

-- Change rating_notifications.restaurant_id from INTEGER to VARCHAR first
ALTER TABLE rating_notifications MODIFY COLUMN restaurant_id VARCHAR(255);

-- Restore rating_notifications to use name-based restaurant identifiers
ALTER TABLE rating_notifications ADD COLUMN restaurant_id_temp VARCHAR(255);
UPDATE rating_notifications r
INNER JOIN restaurants newr ON r.restaurant_id = newr.id
SET r.restaurant_id_temp = newr.restaurant_code;
UPDATE rating_notifications SET restaurant_id = restaurant_id_temp WHERE restaurant_id_temp IS NOT NULL;
ALTER TABLE rating_notifications DROP COLUMN restaurant_id_temp;

-- Remove duplicated restaurants created during upgrade (keep one per restaurant_code)
DELETE FROM restaurants 
WHERE id NOT IN (
    SELECT MIN(id) 
    FROM restaurants 
    GROUP BY restaurant_code
);

ALTER TABLE restaurants DROP FOREIGN KEY IF EXISTS fk_restaurants_group_id;
ALTER TABLE restaurants DROP COLUMN IF EXISTS id;
ALTER TABLE restaurants CHANGE COLUMN restaurant_code id VARCHAR(255) NOT NULL;
ALTER TABLE restaurants DROP COLUMN group_id;

-- Re-add foreign key constraints for ratings and rating_notifications
ALTER TABLE ratings ADD FOREIGN KEY fk_ratings_restaurant_id (restaurant_id) REFERENCES restaurants(id);
ALTER TABLE rating_notifications ADD FOREIGN KEY fk_rating_notifications_restaurant_id (restaurant_id) REFERENCES restaurants(id);