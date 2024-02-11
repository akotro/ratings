-- Add up migration script here
CREATE TABLE users (
    id CHAR(36) PRIMARY KEY,
    username TEXT NOT NULL,
    password TEXT NOT NULL
);

CREATE TABLE restaurants (
    id VARCHAR(255) NOT NULL PRIMARY KEY,
    cuisine VARCHAR(255) NOT NULL
);

INSERT INTO restaurants (id, cuisine)
VALUES
    ('ARMYRA BY PAPAIOANNOU', 'Seafood'),
    ('BARBOUNI BAR RESTAURANT', 'Seafood'),
    ('DA LUIGI RESTAURANT', 'Italian'),
    ('FLAME RESTAURANT', 'Steakhouse'),
    ('ONUKI', 'Japanese'),
    ('KAFENIO & DELI', 'Greek Cafe'),
    ('KOOC TAVERNA SECRETS', 'Greek Fusion'),
    ('MORIAS RESTAURANT', 'Breakfast'),
    ('NARGILE', 'Lebanese'),
    ('PERO RESTAURANT', 'Breakfast'),
    ('SOUVLAKERIE', 'Greek Souvlaki');

CREATE TABLE menu_items (
    id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    price FLOAT NOT NULL
);

CREATE TABLE restaurant_menu_items (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    restaurant_id VARCHAR(255) NOT NULL,
    menu_item_id INTEGER NOT NULL,
    FOREIGN KEY (restaurant_id) REFERENCES restaurants(id),
    FOREIGN KEY (menu_item_id) REFERENCES menu_items(id)
);

CREATE TABLE ratings (
    id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
    restaurant_id VARCHAR(255) NOT NULL,
    user_id CHAR(36) NOT NULL,
    score FLOAT NOT NULL,
    FOREIGN KEY (restaurant_id) REFERENCES restaurants(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE ip_blacklist (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    ip_address VARCHAR(15) NOT NULL,
    UNIQUE (ip_address)
);

INSERT INTO ip_blacklist (ip_address)
VALUES
    ('103.178.228.27'),
    ('111.173.118.83'),
    ('111.85.200.64'),
    ('114.100.177.182'),
    ('119.60.105.179'),
    ('124.90.215.107'),
    ('139.170.202.166'),
    ('144.255.16.185'),
    ('152.89.196.144'),
    ('159.203.192.14'),
    ('159.203.208.12'),
    ('162.142.125.215'),
    ('162.243.151.4'),
    ('167.94.146.58'),
    ('167.99.141.170'),
    ('171.118.64.11'),
    ('182.54.4.215'),
    ('183.136.225.9'),
    ('184.105.139.124'),
    ('184.105.139.72'),
    ('184.105.139.76'),
    ('185.180.143.188'),
    ('185.180.143.50'),
    ('188.26.198.163'),
    ('192.155.90.118'),
    ('198.235.24.205'),
    ('198.235.24.215'),
    ('20.187.65.106'),
    ('216.146.25.63'),
    ('219.143.174.49'),
    ('3.239.85.113'),
    ('74.234.160.134'),
    ('78.108.177.51'),
    ('80.71.157.4'),
    ('87.236.176.161'),
    ('94.183.49.9'),
    ('94.236.135.131'),
    ('95.214.53.99'),
    ('50.116.2.74');
