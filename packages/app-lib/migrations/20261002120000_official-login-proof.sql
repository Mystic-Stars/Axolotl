CREATE TABLE official_login_proof (
	id INTEGER PRIMARY KEY CHECK (id = 0),
	verified INTEGER NOT NULL CHECK (verified = 1),
	version INTEGER NOT NULL CHECK (version = 1),
	mac TEXT NOT NULL
);
