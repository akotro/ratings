-- Add down migration script here
DELETE FROM restaurants
WHERE id IN (
    'BERYTUS',
    'BRASSERIE DE LA BAY',
    'GUACANA',
    'HAMS & CLAMS',
    'MINCO STORIES',
    'OLIVIERA',
    'PIZZA SAPIENZA',
    'SANTORRE',
    'TAHIR'
);
