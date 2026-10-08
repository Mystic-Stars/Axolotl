UPDATE settings
SET download_engine = 'legacy';

UPDATE settings
SET feature_flags = json_remove(feature_flags, '$.xmcl_download_engine');
