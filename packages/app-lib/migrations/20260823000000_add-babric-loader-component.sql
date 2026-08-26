DROP INDEX instance_loader_components_primary;
DROP INDEX instance_loader_components_kind;

ALTER TABLE instance_loader_components RENAME TO instance_loader_components_old;

CREATE TABLE instance_loader_components (
	instance_id TEXT NOT NULL,
	kind TEXT NOT NULL CHECK(kind IN (
		'vanilla',
		'forge',
		'neoforge',
		'fabric',
		'quilt',
		'cleanroom',
		'legacy_fabric',
		'babric',
		'optifine',
		'lite_loader',
		'optifabric'
	)),
	version TEXT NULL,
	role TEXT NOT NULL CHECK(role IN ('primary', 'adjunct')),
	provider_metadata TEXT NULL
		CHECK(provider_metadata IS NULL OR json_valid(provider_metadata)),

	PRIMARY KEY (instance_id, kind),
	FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);

INSERT INTO instance_loader_components (
	instance_id, kind, version, role, provider_metadata
)
SELECT instance_id, kind, version, role, provider_metadata
FROM instance_loader_components_old;

DROP TABLE instance_loader_components_old;

CREATE UNIQUE INDEX instance_loader_components_primary
	ON instance_loader_components(instance_id)
	WHERE role = 'primary';

CREATE INDEX instance_loader_components_kind
	ON instance_loader_components(kind);
