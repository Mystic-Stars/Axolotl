CREATE TABLE discovered_javas (
    path TEXT NOT NULL PRIMARY KEY,
    major_version INTEGER NOT NULL,
    full_version TEXT NOT NULL,
    architecture TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_mtime_ms INTEGER NOT NULL
);

CREATE INDEX discovered_javas_major_version ON discovered_javas (major_version);
