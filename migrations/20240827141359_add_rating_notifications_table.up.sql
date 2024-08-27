-- Add up migration script here
CREATE TABLE rating_notifications (
    id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
    group_id CHAR(36) NOT NULL,
    restaurant_id VARCHAR(255) NOT NULL,
    notified_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY unique_notification (group_id, restaurant_id),
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
    FOREIGN KEY (restaurant_id) REFERENCES restaurants(id) ON DELETE CASCADE
);
