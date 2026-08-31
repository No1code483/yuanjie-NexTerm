-- Rollback migration v115
ALTER TABLE conversations DROP COLUMN sort_order;
ALTER TABLE conversations DROP COLUMN unread_count;
