ALTER TABLE request_logs ADD COLUMN requested_model TEXT;
ALTER TABLE request_logs ADD COLUMN model_fallback_path_json TEXT;
