ALTER TABLE accounts ADD COLUMN note TEXT;
ALTER TABLE accounts ADD COLUMN tags TEXT;
ALTER TABLE accounts ADD COLUMN group_name TEXT;

ALTER TABLE login_sessions ADD COLUMN note TEXT;
ALTER TABLE login_sessions ADD COLUMN tags TEXT;
ALTER TABLE login_sessions ADD COLUMN group_name TEXT;
