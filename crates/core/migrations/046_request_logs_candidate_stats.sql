ALTER TABLE request_logs ADD COLUMN candidate_count INTEGER;
ALTER TABLE request_logs ADD COLUMN attempted_count INTEGER;
ALTER TABLE request_logs ADD COLUMN skipped_count INTEGER;
ALTER TABLE request_logs ADD COLUMN skipped_cooldown_count INTEGER;
ALTER TABLE request_logs ADD COLUMN skipped_inflight_count INTEGER;
