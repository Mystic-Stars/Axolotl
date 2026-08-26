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

CREATE UNIQUE INDEX instance_loader_components_primary
	ON instance_loader_components(instance_id)
	WHERE role = 'primary';

CREATE INDEX instance_loader_components_kind
	ON instance_loader_components(kind);

INSERT INTO instance_loader_components (
	instance_id,
	kind,
	version,
	role,
	provider_metadata
)
SELECT
	i.id,
	CASE cs.loader
		WHEN 'optifine' THEN 'vanilla'
		WHEN 'lite_loader' THEN 'vanilla'
		ELSE cs.loader
	END,
	CASE
		WHEN cs.loader IN ('optifine', 'lite_loader') THEN NULL
		ELSE cs.loader_version
	END,
	'primary',
	NULL
FROM instances i
INNER JOIN instance_content_sets cs
	ON cs.id = i.applied_content_set_id
	AND cs.instance_id = i.id
WHERE cs.loader IN (
	'vanilla',
	'forge',
	'neoforge',
	'fabric',
	'quilt',
	'cleanroom',
	'legacy_fabric',
	'optifine',
	'lite_loader'
);

INSERT INTO instance_loader_components (
	instance_id,
	kind,
	version,
	role,
	provider_metadata
)
SELECT
	i.id,
	cs.loader,
	cs.loader_version,
	'adjunct',
	NULL
FROM instances i
INNER JOIN instance_content_sets cs
	ON cs.id = i.applied_content_set_id
	AND cs.instance_id = i.id
WHERE cs.loader IN ('optifine', 'lite_loader');
