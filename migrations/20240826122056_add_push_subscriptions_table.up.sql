-- Add up migration script here
CREATE TABLE push_subscriptions (
    endpoint VARCHAR(255) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
