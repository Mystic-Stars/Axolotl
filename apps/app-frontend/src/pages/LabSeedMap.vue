<script setup lang="ts">
import {
	ChevronDownIcon,
	CircleAlertIcon,
	ClipboardCopyIcon,
	CompassIcon,
	ContractIcon,
	ExpandIcon,
	EyeIcon,
	EyeOffIcon,
	GridIcon,
	HashIcon,
	HistoryIcon,
	ImportIcon,
	LandmarkIcon,
	LayersIcon,
	MinusIcon,
	PickaxeIcon,
	PinIcon,
	PlusIcon,
	RefreshCwIcon,
	ScaleIcon,
	SearchIcon,
	SettingsIcon,
	ShareIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	Checkbox,
	defineMessages,
	DropdownSelect,
	injectNotificationManager,
	PopoutMenu,
	Slider,
	StyledInput,
	Toggle,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'
import {
	computed,
	nextTick,
	onMounted,
	onUnmounted,
	reactive,
	ref,
	useTemplateRef,
	watch,
} from 'vue'
import { useRoute } from 'vue-router'

import SeedMapBiomePicker from '@/components/lab/seed-map/SeedMapBiomePicker.vue'
import SeedMapCopyrightModal from '@/components/lab/seed-map/SeedMapCopyrightModal.vue'
import SeedMapWorldImportModal, {
	type SeedMapWorldImport,
} from '@/components/lab/seed-map/SeedMapWorldImportModal.vue'
import {
	applyShareQuery,
	clearSeedMapHistory,
	createDefaultSeedMapWorkspace,
	createShareQuery,
	fallbackSeedMapProfiles,
	featureKey,
	featureMask,
	findSeedMapFeatures,
	getSeedMapBiomeAt,
	getSeedMapProfiles,
	getSeedMapSpawn,
	isCurrentSeedMapEpoch,
	loadSeedMapHistory,
	loadSeedMapWorkspace,
	recordSeedMapHistory,
	removeSeedMapHistoryEntry,
	renderSeedMapTile,
	saveSeedMapWorkspace,
	SEED_MAP_BIOME_NAMES,
	SEED_MAP_BIOMES,
	SEED_MAP_END_CITY_IMAGE_SOURCES as endCityImageSources,
	SEED_MAP_FEATURE_COLORS as featureColors,
	SEED_MAP_FEATURE_ICONS as featureIcons,
	SEED_MAP_FEATURE_IMAGE_SOURCES as featureImageSources,
	SEED_MAP_FEATURES,
	SEED_MAP_MIN_ZOOM,
	SEED_MAP_ORE_MAX_SCALE,
	SEED_MAP_SCALES,
	SEED_MAP_STRUCTURE_ASSET_ROOT as structureAssetRoot,
	seedMapBiomeSlug,
	type SeedMapDimension,
	type SeedMapDisplayMode,
	type SeedMapEdition,
	type SeedMapFeature,
	type SeedMapFeatureKind,
	type SeedMapHistoryEntry,
	seedMapHistoryId,
	type SeedMapHistorySource,
	type SeedMapMarker,
	seedMapOreDefinition,
	type SeedMapOreHit,
	seedMapOreKey,
	type SeedMapOreKind,
	seedMapOresForDimension,
	seedMapOreYRange,
	type SeedMapSpawnPoint,
	seedMapTileConcurrency,
	type SeedMapTileRequest,
	type SeedMapVersionProfile,
	updateSeedMapHistoryProgress,
	useSeedMapOreLayer,
	visibleFeatureDefinitions,
} from '@/lab/seed-map'

type MapBounds = {
	minX: number
	minZ: number
	maxX: number
	maxZ: number
}

type Selection = {
	x: number
	z: number
	feature?: SeedMapFeature
	ore?: SeedMapOreHit
	markerId?: string
	spawn?: boolean
	biome?: number
}

type TileJob = {
	epoch: number
	key: string
	run: () => Promise<void>
}

type ActivePointer = {
	x: number
	y: number
}

const { addNotification, handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const route = useRoute()
const canvas = useTemplateRef<HTMLCanvasElement>('mapCanvas')
const fullscreenContainer = useTemplateRef<HTMLElement>('fullscreenContainer')
const workspace = reactive(applyShareQuery(route.query, loadSeedMapWorkspace()))
const profiles = ref<SeedMapVersionProfile[]>(fallbackSeedMapProfiles())
const features = ref<SeedMapFeature[]>([])
const spawn = ref<SeedMapSpawnPoint | null>(null)
const selection = ref<Selection | null>(null)
const markerName = ref('')
const markerColor = ref('#22C55E')
const markerDraftOpen = ref(false)
const advancedOpen = ref(false)
const layersExpanded = ref(false)
const showLayerNames = ref(false)
const rulerPoints = ref<{ x: number; z: number }[]>([])
const rulerEnabled = ref(false)
const isFullscreen = ref(false)
const loading = ref(false)
const mapError = ref('')
const historyEntries = ref<SeedMapHistoryEntry[]>(loadSeedMapHistory())
const worldImportModal =
	useTemplateRef<InstanceType<typeof SeedMapWorldImportModal>>('worldImportModal')
const copyrightModal = useTemplateRef<InstanceType<typeof SeedMapCopyrightModal>>('copyrightModal')
let activeHistoryId = seedMapHistoryId(workspace.seed.trim().slice(0, 256), workspace.edition)
const elevationInput = ref(String(workspace.elevation))
const coordinateX = ref(String(Math.round(workspace.center.x)))
const coordinateZ = ref(String(Math.round(workspace.center.z)))
const tileImages = new Map<string, ImageBitmap>()
const featureImages = new Map<string, HTMLImageElement>()
const oreImages = new Map<string, HTMLImageElement>()
let spawnImage: HTMLImageElement | undefined
const pendingTiles = new Map<string, number>()
const tileQueue: TileJob[] = []
const activePointers = new Map<number, ActivePointer>()
const pointer = reactive({ id: -1, startX: 0, startY: 0, centerX: 0, centerZ: 0, moved: false })
const viewport = reactive({ width: 0, height: 0 })
let activeTileRequests = 0
const activeTileRequestsByEpoch = new Map<number, number>()
let requestEpoch = 0
let refreshTimer: ReturnType<typeof window.setTimeout> | undefined
let saveTimer: ReturnType<typeof window.setTimeout> | undefined
let longPressTimer: ReturnType<typeof window.setTimeout> | undefined
let redrawFrame: number | undefined
let panFrame: number | undefined
let pendingPanCenter: { x: number; z: number } | undefined
let zoomAnimationFrame: number | undefined
let fallbackTileScale: number | undefined
let resizeObserver: ResizeObserver | undefined
let pinchDistance = 0
let wheelMode: 'trackpad' | 'wheel' | undefined
let wheelGestureStartedAt = 0
let wheelDelta = 0
let wheelTimer: ReturnType<typeof window.setTimeout> | undefined
let trackpadTimer: ReturnType<typeof window.setTimeout> | undefined
let wheelAnchor = { x: 0, y: 0 }
let suppressPointerSelection = false
let centerOnNextSpawn = false
let spawnContextKey = ''
let pendingHistorySource: SeedMapHistorySource | null = null
let pendingHistoryLabels: { instanceName?: string; worldName?: string } = {}
let historyTimer: ReturnType<typeof window.setTimeout> | undefined
let progressTimer: ReturnType<typeof window.setTimeout> | undefined
let applyingSharedState = false
let biomeLookupToken = 0

const formatRelativeTime = useRelativeTime()

const {
	hits: oreHits,
	scanning: oreScanning,
	scannedChunks: oreScannedChunks,
	totalChunks: oreTotalChunks,
	refresh: refreshOreLayer,
	refreshFilter: refreshOreFilter,
	dispose: disposeOreLayer,
} = useSeedMapOreLayer({
	onUpdate: () => redraw(),
	onError: () => {
		mapError.value = formatMessage(messages.oreScanFailed)
	},
})

const messages = defineMessages({
	title: { id: 'app.lab.seed-map.title', defaultMessage: 'Seed map' },
	seed: { id: 'app.lab.seed-map.seed', defaultMessage: 'Seed' },
	seedPlaceholder: {
		id: 'app.lab.seed-map.seed-placeholder',
		defaultMessage: 'Enter a number or text seed',
	},
	randomSeed: { id: 'app.lab.seed-map.random-seed', defaultMessage: 'Random seed' },
	importFromInstance: {
		id: 'app.lab.seed-map.import-from-instance',
		defaultMessage: 'Load from instance',
	},
	worldSeedLoaded: {
		id: 'app.lab.seed-map.world-seed-loaded',
		defaultMessage: 'Loaded the seed of {world}',
	},
	history: { id: 'app.lab.seed-map.history', defaultMessage: 'Seed history' },
	historyEmpty: {
		id: 'app.lab.seed-map.history-empty',
		defaultMessage: 'Seeds you view will appear here',
	},
	clearHistory: { id: 'app.lab.seed-map.clear-history', defaultMessage: 'Clear history' },
	removeHistoryEntry: {
		id: 'app.lab.seed-map.remove-history-entry',
		defaultMessage: 'Remove from history',
	},
	historySourceManual: {
		id: 'app.lab.seed-map.history-source.manual',
		defaultMessage: 'Entered manually',
	},
	historySourceRandom: {
		id: 'app.lab.seed-map.history-source.random',
		defaultMessage: 'Random seed',
	},
	historySourceShare: {
		id: 'app.lab.seed-map.history-source.share',
		defaultMessage: 'Shared link',
	},
	edition: { id: 'app.lab.seed-map.edition', defaultMessage: 'Edition' },
	java: { id: 'app.lab.seed-map.edition.java', defaultMessage: 'Java Edition' },
	javaLargeBiomes: {
		id: 'app.lab.seed-map.edition.java-large-biomes',
		defaultMessage: 'Java: Large Biomes',
	},
	gameVersion: { id: 'app.lab.seed-map.version', defaultMessage: 'Game version' },
	dimension: { id: 'app.lab.seed-map.dimension', defaultMessage: 'Dimension' },
	overworld: { id: 'app.lab.seed-map.dimension.overworld', defaultMessage: 'Overworld' },
	nether: { id: 'app.lab.seed-map.dimension.nether', defaultMessage: 'Nether' },
	end: { id: 'app.lab.seed-map.dimension.end', defaultMessage: 'The End' },
	coordinates: { id: 'app.lab.seed-map.coordinates', defaultMessage: 'Coordinates' },
	go: { id: 'app.lab.seed-map.go', defaultMessage: 'Go' },
	layers: { id: 'app.lab.seed-map.layers', defaultMessage: 'Map layers' },
	structures: { id: 'app.lab.seed-map.mode.structures', defaultMessage: 'Structures' },
	ores: { id: 'app.lab.seed-map.mode.ores', defaultMessage: 'Ores' },
	oreLayers: { id: 'app.lab.seed-map.ore-layers', defaultMessage: 'Ore layers' },
	zoomToScanOres: {
		id: 'app.lab.seed-map.zoom-to-scan-ores',
		defaultMessage: 'Zoom in to scan ores',
	},
	noOresInEnd: {
		id: 'app.lab.seed-map.no-ores-in-end',
		defaultMessage: 'Ore scanning is not available in The End',
	},
	oresRequireModern: {
		id: 'app.lab.seed-map.ores-require-modern',
		defaultMessage: 'Ore prediction requires Java 1.18 or newer',
	},
	oreYRange: { id: 'app.lab.seed-map.ore-y-range', defaultMessage: 'Y range' },
	oreMinY: { id: 'app.lab.seed-map.ore-min-y', defaultMessage: 'Minimum Y' },
	oreMaxY: { id: 'app.lab.seed-map.ore-max-y', defaultMessage: 'Maximum Y' },
	resetYRange: { id: 'app.lab.seed-map.reset-y-range', defaultMessage: 'Reset Y range' },
	scanningOres: {
		id: 'app.lab.seed-map.scanning-ores',
		defaultMessage: 'Scanning ores {processed}/{total}',
	},
	oreScanFailed: {
		id: 'app.lab.seed-map.ore-scan-failed',
		defaultMessage: 'Ore scanning failed. Try refreshing the map.',
	},
	oreVerified: { id: 'app.lab.seed-map.ore-verified', defaultMessage: 'Verified' },
	oreLikely: { id: 'app.lab.seed-map.ore-likely', defaultMessage: 'Likely' },
	orePrecision: {
		id: 'app.lab.seed-map.ore-precision',
		defaultMessage: '{precision}% precision',
	},
	oreCoverage: {
		id: 'app.lab.seed-map.ore-coverage',
		defaultMessage: 'Y {min} to {max}',
	},
	mined: { id: 'app.lab.seed-map.mined', defaultMessage: 'Mined' },
	expandLayers: { id: 'app.lab.seed-map.expand-layers', defaultMessage: 'Expand layers' },
	collapseLayers: { id: 'app.lab.seed-map.collapse-layers', defaultMessage: 'Collapse layers' },
	showNames: { id: 'app.lab.seed-map.show-layer-names', defaultMessage: 'Show layer names' },
	hideNames: { id: 'app.lab.seed-map.hide-layer-names', defaultMessage: 'Hide layer names' },
	selectAll: { id: 'app.lab.seed-map.select-all', defaultMessage: 'Select all' },
	clear: { id: 'app.lab.seed-map.clear', defaultMessage: 'Clear' },
	reset: { id: 'app.lab.seed-map.reset', defaultMessage: 'Reset' },
	spawnPoint: { id: 'app.lab.seed-map.spawn-point', defaultMessage: 'Spawn point' },
	zoomToShow: { id: 'app.lab.seed-map.zoom-to-show', defaultMessage: 'Zoom in to show {layer}' },
	mapSettings: { id: 'app.lab.seed-map.settings', defaultMessage: 'Advanced settings' },
	showGrid: { id: 'app.lab.seed-map.show-grid', defaultMessage: 'Grid' },
	chunkCoordinates: {
		id: 'app.lab.seed-map.chunk-coordinates',
		defaultMessage: 'Chunk coordinates',
	},
	terrain: { id: 'app.lab.seed-map.terrain', defaultMessage: 'Terrain' },
	contours: { id: 'app.lab.seed-map.contours', defaultMessage: 'Contour lines' },
	elevation: { id: 'app.lab.seed-map.elevation', defaultMessage: 'Surface' },
	elevationLockedByTerrain: {
		id: 'app.lab.seed-map.elevation-locked',
		defaultMessage: 'Follows the estimated surface while terrain estimation is on',
	},
	zoomIn: { id: 'app.lab.seed-map.zoom-in', defaultMessage: 'Zoom in' },
	zoomOut: { id: 'app.lab.seed-map.zoom-out', defaultMessage: 'Zoom out' },
	fullscreen: { id: 'app.lab.seed-map.fullscreen', defaultMessage: 'Fullscreen' },
	exitFullscreen: { id: 'app.lab.seed-map.exit-fullscreen', defaultMessage: 'Exit fullscreen' },
	centerSpawn: { id: 'app.lab.seed-map.center-spawn', defaultMessage: 'Center spawn' },
	searchNearby: { id: 'app.lab.seed-map.search-nearby', defaultMessage: 'Nearest structure' },
	location: { id: 'app.lab.seed-map.location', defaultMessage: 'Location' },
	biomeTag: { id: 'app.lab.seed-map.biome-tag', defaultMessage: 'Biome' },
	copyTeleport: { id: 'app.lab.seed-map.copy-teleport', defaultMessage: 'Copy /tp' },
	teleportCopied: {
		id: 'app.lab.seed-map.teleport-copied',
		defaultMessage: 'Teleport command copied',
	},
	addMarker: { id: 'app.lab.seed-map.add-marker', defaultMessage: 'Add marker' },
	markerName: { id: 'app.lab.seed-map.marker-name', defaultMessage: 'Marker name' },
	markerColor: { id: 'app.lab.seed-map.marker-color', defaultMessage: 'Marker color' },
	markerSaved: { id: 'app.lab.seed-map.marker-saved', defaultMessage: 'Marker saved' },
	markers: { id: 'app.lab.seed-map.markers', defaultMessage: 'My markers' },
	noMarkers: { id: 'app.lab.seed-map.no-markers', defaultMessage: 'No saved markers' },
	removeMarker: { id: 'app.lab.seed-map.remove-marker', defaultMessage: 'Remove marker' },
	completed: { id: 'app.lab.seed-map.completed', defaultMessage: 'Completed' },
	ruler: { id: 'app.lab.seed-map.ruler', defaultMessage: 'Measure distance' },
	distance: { id: 'app.lab.seed-map.distance', defaultMessage: '{distance} blocks' },
	clearRuler: { id: 'app.lab.seed-map.clear-ruler', defaultMessage: 'Clear measurement' },
	share: { id: 'app.lab.seed-map.share', defaultMessage: 'Share' },
	shareCopied: { id: 'app.lab.seed-map.share-copied', defaultMessage: 'Map link copied' },
	approximate: { id: 'app.lab.seed-map.approximate', defaultMessage: 'Estimated' },
	mapScale: { id: 'app.lab.seed-map.map-scale', defaultMessage: '1 px = {scale} blocks' },
	copyright: {
		id: 'app.lab.seed-map.copyright.open',
		defaultMessage: 'Copyright and attribution',
	},
	loading: { id: 'app.lab.seed-map.loading', defaultMessage: 'Loading map' },
	mapFailed: { id: 'app.lab.seed-map.map-failed', defaultMessage: 'Map generation failed' },
	closePanel: { id: 'app.lab.seed-map.close-panel', defaultMessage: 'Close panel' },
	village: { id: 'app.lab.seed-map.feature.village', defaultMessage: 'Village' },
	outpost: { id: 'app.lab.seed-map.feature.outpost', defaultMessage: 'Pillager outpost' },
	shipwreck: { id: 'app.lab.seed-map.feature.shipwreck', defaultMessage: 'Shipwreck' },
	monument: { id: 'app.lab.seed-map.feature.monument', defaultMessage: 'Ocean monument' },
	mansion: { id: 'app.lab.seed-map.feature.mansion', defaultMessage: 'Woodland mansion' },
	ancientCity: { id: 'app.lab.seed-map.feature.ancient-city', defaultMessage: 'Ancient city' },
	trailRuins: { id: 'app.lab.seed-map.feature.trail-ruins', defaultMessage: 'Trail ruins' },
	trialChambers: {
		id: 'app.lab.seed-map.feature.trial-chambers',
		defaultMessage: 'Trial chambers',
	},
	ruinedPortal: { id: 'app.lab.seed-map.feature.ruined-portal', defaultMessage: 'Ruined portal' },
	stronghold: { id: 'app.lab.seed-map.feature.stronghold', defaultMessage: 'Stronghold' },
	slimeChunk: { id: 'app.lab.seed-map.feature.slime-chunk', defaultMessage: 'Slime chunks' },
	desertPyramid: {
		id: 'app.lab.seed-map.feature.desert-pyramid',
		defaultMessage: 'Desert pyramid',
	},
	jungleTemple: { id: 'app.lab.seed-map.feature.jungle-temple', defaultMessage: 'Jungle temple' },
	swampHut: { id: 'app.lab.seed-map.feature.swamp-hut', defaultMessage: 'Swamp hut' },
	igloo: { id: 'app.lab.seed-map.feature.igloo', defaultMessage: 'Igloo' },
	oceanRuin: { id: 'app.lab.seed-map.feature.ocean-ruin', defaultMessage: 'Ocean ruin' },
	buriedTreasure: {
		id: 'app.lab.seed-map.feature.buried-treasure',
		defaultMessage: 'Buried treasure',
	},
	mineshaft: { id: 'app.lab.seed-map.feature.mineshaft', defaultMessage: 'Mineshaft' },
	desertWell: { id: 'app.lab.seed-map.feature.desert-well', defaultMessage: 'Desert well' },
	geode: { id: 'app.lab.seed-map.feature.geode', defaultMessage: 'Amethyst geode' },
	fortress: { id: 'app.lab.seed-map.feature.fortress', defaultMessage: 'Nether fortress' },
	bastion: { id: 'app.lab.seed-map.feature.bastion', defaultMessage: 'Bastion remnant' },
	endCity: { id: 'app.lab.seed-map.feature.end-city', defaultMessage: 'End city' },
	endCityWithShip: {
		id: 'app.lab.seed-map.feature.end-city-with-ship',
		defaultMessage: 'End city (with ship)',
	},
	endCityWithoutShip: {
		id: 'app.lab.seed-map.feature.end-city-without-ship',
		defaultMessage: 'End city (without ship)',
	},
	endGateway: { id: 'app.lab.seed-map.feature.end-gateway', defaultMessage: 'End gateway' },
	oreDiamond: { id: 'app.lab.seed-map.ore.diamond', defaultMessage: 'Diamond ore' },
	oreNetherite: { id: 'app.lab.seed-map.ore.netherite', defaultMessage: 'Ancient debris' },
	oreIron: { id: 'app.lab.seed-map.ore.iron', defaultMessage: 'Iron ore' },
	oreIronVein: { id: 'app.lab.seed-map.ore.iron-vein', defaultMessage: 'Iron vein' },
	oreCopper: { id: 'app.lab.seed-map.ore.copper', defaultMessage: 'Copper ore' },
	oreCopperVein: { id: 'app.lab.seed-map.ore.copper-vein', defaultMessage: 'Copper vein' },
	oreGold: { id: 'app.lab.seed-map.ore.gold', defaultMessage: 'Gold ore' },
	oreRedstone: { id: 'app.lab.seed-map.ore.redstone', defaultMessage: 'Redstone ore' },
	oreLapis: { id: 'app.lab.seed-map.ore.lapis', defaultMessage: 'Lapis lazuli ore' },
	oreCoal: { id: 'app.lab.seed-map.ore.coal', defaultMessage: 'Coal ore' },
})

const featureMessages: Record<SeedMapFeatureKind, (typeof messages)[keyof typeof messages]> = {
	village: messages.village,
	outpost: messages.outpost,
	shipwreck: messages.shipwreck,
	monument: messages.monument,
	mansion: messages.mansion,
	'ancient-city': messages.ancientCity,
	'trail-ruins': messages.trailRuins,
	'trial-chambers': messages.trialChambers,
	'ruined-portal': messages.ruinedPortal,
	stronghold: messages.stronghold,
	'slime-chunk': messages.slimeChunk,
	'desert-pyramid': messages.desertPyramid,
	'jungle-temple': messages.jungleTemple,
	'swamp-hut': messages.swampHut,
	igloo: messages.igloo,
	'ocean-ruin': messages.oceanRuin,
	'buried-treasure': messages.buriedTreasure,
	mineshaft: messages.mineshaft,
	'desert-well': messages.desertWell,
	geode: messages.geode,
	fortress: messages.fortress,
	bastion: messages.bastion,
	'end-city': messages.endCity,
	'end-gateway': messages.endGateway,
}

const oreMessages: Record<SeedMapOreKind, (typeof messages)[keyof typeof messages]> = {
	diamond: messages.oreDiamond,
	netherite: messages.oreNetherite,
	iron: messages.oreIron,
	iron_vein: messages.oreIronVein,
	copper: messages.oreCopper,
	copper_vein: messages.oreCopperVein,
	gold: messages.oreGold,
	redstone: messages.oreRedstone,
	lapis: messages.oreLapis,
	coal: messages.oreCoal,
}

const editionOptions: SeedMapEdition[] = ['java', 'java-large-biomes']
const selectedProfile = computed(() =>
	profiles.value.find(
		(profile) => profile.edition === workspace.edition && profile.version === workspace.gameVersion,
	),
)
const availableVersions = computed(() =>
	profiles.value
		.filter((profile) => profile.edition === workspace.edition)
		.map((profile) => profile.version),
)
const dimensions = computed(
	() => selectedProfile.value?.dimensions ?? ['overworld', 'nether', 'end'],
)
const scale = computed(() => 4 ** workspace.zoom)
const tileScale = computed(
	() =>
		SEED_MAP_SCALES[Math.min(Math.max(Math.round(workspace.zoom), 0), SEED_MAP_SCALES.length - 1)],
)
const mapScaleLabel = computed(() =>
	scale.value >= 10 ? String(Math.round(scale.value)) : scale.value.toFixed(2).replace(/\.00$/, ''),
)
const dimensionFeatures = computed(() =>
	SEED_MAP_FEATURES.filter((feature) => feature.dimensions.includes(workspace.dimension)),
)
const requestedFeatures = computed(() =>
	visibleFeatureDefinitions(workspace.visibleFeatures, workspace.dimension, tileScale.value),
)
const requestedFeatureKinds = computed(
	() => new Set(requestedFeatures.value.map((feature) => feature.kind)),
)
const displayedFeatures = computed(() =>
	features.value.filter((feature) => requestedFeatureKinds.value.has(feature.kind)),
)
const activeFeatureMask = computed(() =>
	featureMask(requestedFeatures.value.map((feature) => feature.kind)),
)
const oreModeSupported = computed(() => selectedProfile.value?.ores !== false)
const dimensionOres = computed(() =>
	oreModeSupported.value ? seedMapOresForDimension(workspace.dimension) : [],
)
const activeSelectedOres = computed(() => {
	const available = new Set(dimensionOres.value.map((ore) => ore.kind))
	return workspace.selectedOres.filter((ore) => available.has(ore))
})
const activeOreYRange = computed(() => seedMapOreYRange(activeSelectedOres.value))
const oreYMinimum = computed({
	get: () => workspace.oreYMin ?? activeOreYRange.value[0],
	set: (value: number) => {
		workspace.oreYMin = Math.min(Math.round(value), workspace.oreYMax ?? activeOreYRange.value[1])
	},
})
const oreYMaximum = computed({
	get: () => workspace.oreYMax ?? activeOreYRange.value[1],
	set: (value: number) => {
		workspace.oreYMax = Math.max(Math.round(value), workspace.oreYMin ?? activeOreYRange.value[0])
	},
})
const oreZoomLimited = computed(
	() => workspace.displayMode === 'ores' && scale.value > SEED_MAP_ORE_MAX_SCALE,
)
const mapBusy = computed(() => loading.value || oreScanning.value)
const availableBiomes = computed(() =>
	SEED_MAP_BIOMES.filter((biome) => biome.dimensions.includes(workspace.dimension)).map(
		(biome) => biome.id,
	),
)
const activeHighlightedBiomes = computed(() =>
	workspace.highlightedBiomes.filter((biome) => availableBiomes.value.includes(biome)),
)
const terrainSupported = computed(() => workspace.dimension !== 'nether')
const terrainEnabled = computed({
	get: () => terrainSupported.value && workspace.terrainEstimation,
	set: (enabled: boolean) => {
		if (!terrainSupported.value) return
		workspace.terrainEstimation = enabled
		if (!enabled) workspace.contourLines = false
	},
})
const contoursEnabled = computed({
	get: () => workspace.contourLines,
	set: (enabled: boolean) => {
		workspace.contourLines = enabled
		if (enabled) workspace.terrainEstimation = true
	},
})
const selectedFeatureKey = computed(() =>
	selection.value?.feature ? featureKey(selection.value.feature) : null,
)
const selectedFeatureCompleted = computed({
	get: () => {
		const key = selectedFeatureKey.value
		return key !== null && workspace.completedFeatures.includes(key)
	},
	set: (completed: boolean) => {
		const key = selectedFeatureKey.value
		if (!key) return
		workspace.completedFeatures = completed
			? [...new Set([...workspace.completedFeatures, key])]
			: workspace.completedFeatures.filter((item) => item !== key)
	},
})
const selectedOreKey = computed(() =>
	selection.value?.ore ? seedMapOreKey(selection.value.ore) : null,
)
const selectedOreCompleted = computed({
	get: () => {
		const key = selectedOreKey.value
		return key !== null && workspace.completedOres.includes(key)
	},
	set: (completed: boolean) => {
		const key = selectedOreKey.value
		if (!key) return
		workspace.completedOres = completed
			? [...new Set([...workspace.completedOres, key])]
			: workspace.completedOres.filter((item) => item !== key)
	},
})
const selectedMarker = computed(() =>
	selection.value?.markerId
		? (workspace.markers.find((marker) => marker.id === selection.value?.markerId) ?? null)
		: null,
)
const rulerDistance = computed(() => {
	if (rulerPoints.value.length !== 2) return null
	const [start, end] = rulerPoints.value
	return Math.round(Math.hypot(end.x - start.x, end.z - start.z))
})
const selectionStyle = computed(() => {
	if (!selection.value) return undefined
	const point = worldToScreen(selection.value.x, selection.value.z)
	return {
		left: `${Math.min(Math.max(point.x + 14, 8), Math.max(viewport.width - 286, 8))}px`,
		top: `${Math.min(Math.max(point.y + 14, 8), Math.max(viewport.height - 226, 8))}px`,
	}
})
const selectionVisible = computed(() => {
	if (!selection.value) return false
	if (selection.value.feature && !requestedFeatureKinds.value.has(selection.value.feature.kind))
		return false
	if (selection.value.ore && workspace.displayMode !== 'ores') return false
	const point = worldToScreen(selection.value.x, selection.value.z)
	return (
		point.x >= -24 &&
		point.x <= viewport.width + 24 &&
		point.y >= -24 &&
		point.y <= viewport.height + 24
	)
})

watch(
	workspace,
	() => {
		if (saveTimer) window.clearTimeout(saveTimer)
		saveTimer = window.setTimeout(() => saveSeedMapWorkspace(workspace), 350)
	},
	{ deep: true },
)

watch(
	() => workspace.seed,
	(seed, previousSeed) => {
		if (seed === previousSeed) return
		stashActiveProgress()
		activeHistoryId = seedMapHistoryId(seed.trim().slice(0, 256), workspace.edition)
		const entry = historyEntries.value.find((item) => item.id === activeHistoryId)
		workspace.completedFeatures = entry ? [...entry.completedFeatures] : []
		workspace.completedOres = entry ? [...entry.completedOres] : []
		const source = pendingHistorySource ?? 'manual'
		const labels = pendingHistoryLabels
		pendingHistorySource = null
		pendingHistoryLabels = {}
		if (historyTimer) window.clearTimeout(historyTimer)
		if (seed.trim()) {
			if (source === 'manual') {
				historyTimer = window.setTimeout(() => {
					historyTimer = undefined
					recordHistory('manual')
				}, 2_000)
			} else {
				recordHistory(source, labels)
			}
		}
		if (applyingSharedState) {
			spawn.value = null
			spawnContextKey = ''
			features.value = []
			return
		}
		const defaults = createDefaultSeedMapWorkspace()
		centerOnNextSpawn = true
		workspace.dimension = 'overworld'
		workspace.center = { x: 0, z: 0 }
		workspace.zoom = defaults.zoom
		workspace.showSpawn = true
		workspace.visibleFeatures = [...defaults.visibleFeatures]
		spawn.value = null
		spawnContextKey = ''
		features.value = []
	},
)

watch(
	() => route.query,
	(query) => {
		if (typeof query.seed !== 'string' || !query.seed.trim()) return
		const shared = applyShareQuery(query, {
			...createDefaultSeedMapWorkspace(),
			markers: [...workspace.markers],
		})
		applyingSharedState = true
		pendingHistorySource = 'share'
		const seedChanged = workspace.seed !== shared.seed
		workspace.edition = shared.edition
		workspace.gameVersion = shared.gameVersion
		workspace.dimension = shared.dimension
		workspace.displayMode = shared.displayMode
		workspace.selectedOres = [...shared.selectedOres]
		workspace.oreYMin = shared.oreYMin
		workspace.oreYMax = shared.oreYMax
		workspace.zoom = shared.zoom
		workspace.center = { ...shared.center }
		workspace.seed = shared.seed
		selection.value = null
		centerOnNextSpawn = false
		if (!seedChanged) {
			pendingHistorySource = null
			recordHistory('share')
		}
		void nextTick(() => {
			applyingSharedState = false
			scheduleRefresh()
		})
	},
)

watch(
	() => [workspace.completedFeatures, workspace.completedOres],
	() => {
		if (progressTimer) window.clearTimeout(progressTimer)
		progressTimer = window.setTimeout(() => {
			progressTimer = undefined
			stashActiveProgress()
		}, 600)
	},
	{ deep: true },
)

watch(
	() => [
		workspace.seed,
		workspace.edition,
		workspace.gameVersion,
		workspace.dimension,
		workspace.displayMode,
		tileScale.value,
		workspace.elevation,
		workspace.terrainEstimation,
		workspace.contourLines,
		workspace.highlightBiomeEnabled,
		workspace.highlightedBiomes.join('|'),
		workspace.showSpawn,
		workspace.visibleFeatures.join('|'),
		workspace.selectedOres.join('|'),
	],
	() => scheduleRefresh(),
	{ flush: 'post' },
)

watch(
	[
		() => workspace.showGrid,
		() => workspace.showChunkCoordinates,
		() => workspace.markers,
		() => workspace.completedFeatures,
		() => workspace.completedOres,
		showLayerNames,
		rulerPoints,
	],
	() => redraw(),
	{ deep: true, flush: 'post' },
)

watch(
	() => [workspace.oreYMin, workspace.oreYMax] as const,
	([yMin, yMax]) => refreshOreFilter(yMin, yMax),
	{ flush: 'post' },
)

watch(
	activeOreYRange,
	([min, max]) => {
		if (workspace.oreYMin !== null)
			workspace.oreYMin = Math.min(max, Math.max(min, workspace.oreYMin))
		if (workspace.oreYMax !== null)
			workspace.oreYMax = Math.min(max, Math.max(min, workspace.oreYMax))
	},
	{ flush: 'post' },
)

watch(
	() => workspace.edition,
	() => {
		const profile = profiles.value.find((item) => item.edition === workspace.edition)
		if (!profile) return
		workspace.gameVersion = profile.version
		workspace.dimension = profile.dimensions.includes(workspace.dimension)
			? workspace.dimension
			: 'overworld'
	},
)

watch(
	() => workspace.gameVersion,
	() => {
		const profile = selectedProfile.value
		if (profile && !profile.dimensions.includes(workspace.dimension))
			workspace.dimension = 'overworld'
		ensureDimensionOreSelection()
	},
)

watch(oreModeSupported, (supported) => {
	if (!supported && workspace.displayMode === 'ores') workspace.displayMode = 'structures'
})

watch(
	() => workspace.dimension,
	(dimension) => {
		if (dimension !== 'overworld') centerOnNextSpawn = false
		if (dimension === 'nether') workspace.contourLines = false
		ensureDimensionOreSelection()
		resetOreYRange()
		selection.value = null
	},
)

watch(
	() => workspace.showSpawn,
	(showSpawn) => {
		if (!showSpawn) centerOnNextSpawn = false
	},
)

watch(
	() => [workspace.center.x, workspace.center.z],
	([x, z]) => {
		coordinateX.value = String(Math.round(x))
		coordinateZ.value = String(Math.round(z))
	},
)

watch(elevationInput, (value) => {
	const parsed = Number(value)
	if (Number.isFinite(parsed))
		workspace.elevation = Math.min(Math.max(Math.round(parsed), -64), 320)
})

onMounted(async () => {
	await nextTick()
	preloadMapIcons()
	resizeCanvas()
	resizeObserver = new ResizeObserver(resizeCanvas)
	if (canvas.value) resizeObserver.observe(canvas.value)
	document.addEventListener('fullscreenchange', onFullscreenChange)
	profiles.value = await getSeedMapProfiles().catch(() => fallbackSeedMapProfiles())
	ensureProfile()
	if (typeof route.query.seed === 'string' && route.query.seed.trim()) {
		recordHistory('share')
	}
	scheduleRefresh(0)
})

onUnmounted(() => {
	if (historyTimer) {
		window.clearTimeout(historyTimer)
		historyTimer = undefined
		recordHistory('manual')
	}
	if (progressTimer) {
		window.clearTimeout(progressTimer)
		progressTimer = undefined
	}
	stashActiveProgress()
	if (refreshTimer) window.clearTimeout(refreshTimer)
	if (saveTimer) window.clearTimeout(saveTimer)
	if (longPressTimer) window.clearTimeout(longPressTimer)
	if (wheelTimer) window.clearTimeout(wheelTimer)
	if (trackpadTimer) window.clearTimeout(trackpadTimer)
	if (redrawFrame !== undefined) window.cancelAnimationFrame(redrawFrame)
	if (panFrame !== undefined) window.cancelAnimationFrame(panFrame)
	if (zoomAnimationFrame !== undefined) window.cancelAnimationFrame(zoomAnimationFrame)
	saveSeedMapWorkspace(workspace)
	disposeOreLayer()
	clearTileImages()
	resizeObserver?.disconnect()
	document.removeEventListener('fullscreenchange', onFullscreenChange)
})

function ensureProfile() {
	if (selectedProfile.value) return
	const replacement =
		profiles.value.find((item) => item.edition === workspace.edition) ?? profiles.value[0]
	if (!replacement) return
	workspace.edition = replacement.edition
	workspace.gameVersion = replacement.version
	workspace.dimension = replacement.dimensions[0] ?? 'overworld'
}

function editionLabel(edition: SeedMapEdition) {
	if (edition === 'java-large-biomes') return formatMessage(messages.javaLargeBiomes)
	return formatMessage(messages.java)
}

function dimensionLabel(dimension: SeedMapDimension) {
	return formatMessage(messages[dimension])
}

function featureLabel(kind: SeedMapFeatureKind) {
	return formatMessage(featureMessages[kind])
}

function featureResultLabel(feature: SeedMapFeature) {
	if (feature.kind !== 'end-city' || feature.endShip === undefined) {
		return featureLabel(feature.kind)
	}
	return formatMessage(feature.endShip ? messages.endCityWithShip : messages.endCityWithoutShip)
}

function featureImageKey(feature: SeedMapFeature) {
	if (feature.kind !== 'end-city' || feature.endShip === undefined) return feature.kind
	return `end-city:${feature.endShip ? 'ship' : 'no-ship'}`
}

function oreLabel(kind: SeedMapOreKind) {
	return formatMessage(oreMessages[kind])
}

function setDisplayMode(mode: SeedMapDisplayMode) {
	if (mode === 'ores' && !oreModeSupported.value) return
	if (mode === 'ores' && workspace.dimension === 'end') workspace.dimension = 'overworld'
	workspace.displayMode = mode
	ensureDimensionOreSelection()
	selection.value = null
}

function ensureDimensionOreSelection() {
	if (dimensionOres.value.length === 0 || activeSelectedOres.value.length > 0) return
	const first = dimensionOres.value[0]?.kind
	if (first) workspace.selectedOres = [...new Set([...workspace.selectedOres, first])]
}

function featureTooltip(kind: SeedMapFeatureKind, maxScale: number) {
	const label = featureLabel(kind)
	return tileScale.value > maxScale ? formatMessage(messages.zoomToShow, { layer: label }) : label
}

function featureImageSource(kind: SeedMapFeatureKind) {
	return featureImageSources[kind]
}

function preloadMapIcons() {
	const preloadFeatureImage = (key: string, source: string) => {
		const image = new Image()
		image.onload = () => {
			featureImages.set(key, image)
			redraw()
		}
		image.src = source
	}
	for (const [kind, source] of Object.entries(featureImageSources) as [
		SeedMapFeatureKind,
		string,
	][]) {
		preloadFeatureImage(kind, source)
	}
	preloadFeatureImage('end-city:ship', endCityImageSources.ship)
	preloadFeatureImage('end-city:no-ship', endCityImageSources.noShip)
	for (const ore of seedMapOresForDimension('overworld').concat(
		seedMapOresForDimension('nether'),
	)) {
		for (const [variant, source] of [
			['stone', ore.texture],
			['deepslate', ore.deepslateTexture],
		] as const) {
			if (!source) continue
			const image = new Image()
			image.onload = () => {
				oreImages.set(`${ore.kind}:${variant}`, image)
				redraw()
			}
			image.src = source
		}
	}
	const image = new Image()
	image.onload = () => {
		spawnImage = image
		redraw()
	}
	image.src = `${structureAssetRoot}/spawn_point.webp`
}

function resizeCanvas() {
	const element = canvas.value
	if (!element) return
	const bounds = element.getBoundingClientRect()
	viewport.width = Math.max(1, Math.round(bounds.width))
	viewport.height = Math.max(1, Math.round(bounds.height))
	const ratio = window.devicePixelRatio || 1
	element.width = Math.round(viewport.width * ratio)
	element.height = Math.round(viewport.height * ratio)
	element.getContext('2d')?.setTransform(ratio, 0, 0, ratio, 0, 0)
	redraw()
	scheduleRefresh()
}

function mapBounds(): MapBounds {
	const halfWidth = (viewport.width * scale.value) / 2
	const halfHeight = (viewport.height * scale.value) / 2
	return {
		minX: Math.floor(workspace.center.x - halfWidth),
		minZ: Math.floor(workspace.center.z - halfHeight),
		maxX: Math.ceil(workspace.center.x + halfWidth),
		maxZ: Math.ceil(workspace.center.z + halfHeight),
	}
}

function worldToScreen(x: number, z: number) {
	const bounds = mapBounds()
	return { x: (x - bounds.minX) / scale.value, y: (z - bounds.minZ) / scale.value }
}

function screenToWorld(x: number, y: number) {
	const bounds = mapBounds()
	return {
		x: Math.round(bounds.minX + x * scale.value),
		z: Math.round(bounds.minZ + y * scale.value),
	}
}

function tileKey(x: number, z: number, sourceScale = tileScale.value) {
	return [
		workspace.seed,
		workspace.edition,
		workspace.gameVersion,
		workspace.dimension,
		sourceScale,
		workspace.elevation,
		terrainEnabled.value,
		workspace.contourLines,
		workspace.highlightBiomeEnabled ? activeHighlightedBiomes.value.join(',') : 'none',
		x,
		z,
	].join(':')
}

function redraw() {
	if (redrawFrame !== undefined) return
	redrawFrame = window.requestAnimationFrame(() => {
		redrawFrame = undefined
		drawMap()
	})
}

function drawMap() {
	const element = canvas.value
	const context = element?.getContext('2d')
	if (!element || !context || !viewport.width || !viewport.height) return
	const style = getComputedStyle(element)
	const surface2 = style.getPropertyValue('--surface-2').trim() || '#17191D'
	const surface3 = style.getPropertyValue('--surface-3').trim() || '#23262D'
	context.clearRect(0, 0, viewport.width, viewport.height)
	context.fillStyle = surface2
	context.fillRect(0, 0, viewport.width, viewport.height)
	context.fillStyle = surface3
	for (let y = 0; y < viewport.height; y += 32) {
		for (let x = 0; x < viewport.width; x += 32) {
			if ((x / 32 + y / 32) % 2 === 0) context.fillRect(x, y, 32, 32)
		}
	}

	const bounds = mapBounds()
	if (fallbackTileScale !== undefined && fallbackTileScale !== tileScale.value) {
		drawTileLayer(context, bounds, fallbackTileScale)
	}
	drawTileLayer(context, bounds, tileScale.value)

	if (workspace.showGrid || workspace.showChunkCoordinates) drawGrid(context, bounds)
	if (workspace.displayMode === 'structures') drawFeatures(context)
	else drawOres(context)
	drawSpawn(context)
	drawMarkers(context)
	drawRuler(context)
}

function drawTileLayer(context: CanvasRenderingContext2D, bounds: MapBounds, sourceScale: number) {
	const tileSpan = 256 * sourceScale
	const minTileX = Math.floor(bounds.minX / tileSpan)
	const minTileZ = Math.floor(bounds.minZ / tileSpan)
	const maxTileX = Math.floor(bounds.maxX / tileSpan)
	const maxTileZ = Math.floor(bounds.maxZ / tileSpan)
	const displaySize = tileSpan / scale.value
	context.save()
	context.imageSmoothingEnabled = false
	for (let tileX = minTileX; tileX <= maxTileX; tileX++) {
		for (let tileZ = minTileZ; tileZ <= maxTileZ; tileZ++) {
			const key = tileKey(tileX, tileZ, sourceScale)
			const bitmap = tileImages.get(key)
			const position = worldToScreen(tileX * tileSpan, tileZ * tileSpan)
			if (bitmap) {
				tileImages.delete(key)
				tileImages.set(key, bitmap)
				context.drawImage(bitmap, position.x, position.y, displaySize, displaySize)
			}
		}
	}
	context.restore()
}

function drawGrid(context: CanvasRenderingContext2D, bounds: MapBounds) {
	const step = Math.max(16, 16 * Math.ceil(scale.value / 16))
	const firstX = Math.floor(bounds.minX / step) * step
	const firstZ = Math.floor(bounds.minZ / step) * step
	context.save()
	context.strokeStyle = 'rgba(255, 255, 255, 0.24)'
	context.lineWidth = 1
	context.font = '11px system-ui'
	context.fillStyle = 'rgba(255, 255, 255, 0.84)'
	for (let x = firstX; x <= bounds.maxX; x += step) {
		const screen = worldToScreen(x, bounds.minZ)
		context.beginPath()
		context.moveTo(screen.x, 0)
		context.lineTo(screen.x, viewport.height)
		context.stroke()
		if (workspace.showChunkCoordinates)
			context.fillText(String(Math.floor(x / 16)), screen.x + 4, 14)
	}
	for (let z = firstZ; z <= bounds.maxZ; z += step) {
		const screen = worldToScreen(bounds.minX, z)
		context.beginPath()
		context.moveTo(0, screen.y)
		context.lineTo(viewport.width, screen.y)
		context.stroke()
		if (workspace.showChunkCoordinates)
			context.fillText(String(Math.floor(z / 16)), 4, screen.y - 4)
	}
	context.restore()
}

function drawFeatures(context: CanvasRenderingContext2D) {
	for (const feature of displayedFeatures.value) {
		const point = worldToScreen(feature.x, feature.z)
		if (
			point.x < -18 ||
			point.x > viewport.width + 18 ||
			point.y < -18 ||
			point.y > viewport.height + 18
		)
			continue
		const selected = selectedFeatureKey.value === featureKey(feature)
		const completed = workspace.completedFeatures.includes(featureKey(feature))
		context.save()
		context.globalAlpha = completed ? 0.48 : 1
		if (feature.kind === 'slime-chunk') {
			const size = Math.max(5, Math.min(16 / scale.value, 14))
			context.fillStyle = featureColors[feature.kind]
			context.strokeStyle = selected ? '#FFFFFF' : 'rgba(15, 18, 20, 0.86)'
			context.lineWidth = selected ? 3 : 2
			context.beginPath()
			context.rect(point.x - size / 2, point.y - size / 2, size, size)
			context.fill()
			context.stroke()
		} else {
			const icon = featureImages.get(featureImageKey(feature))
			if (icon) {
				const size = selected ? 32 : 28
				context.shadowColor = 'rgba(0, 0, 0, 0.48)'
				context.shadowBlur = 5
				context.shadowOffsetY = 2
				context.drawImage(icon, point.x - size / 2, point.y - size / 2, size, size)
				if (selected) {
					context.shadowColor = 'transparent'
					context.strokeStyle = '#FFFFFF'
					context.lineWidth = 2
					context.beginPath()
					context.arc(point.x, point.y, size / 2 + 2, 0, Math.PI * 2)
					context.stroke()
				}
			} else {
				const radius = selected ? 10 : 8
				context.fillStyle = featureColors[feature.kind]
				context.strokeStyle = selected ? '#FFFFFF' : 'rgba(15, 18, 20, 0.86)'
				context.lineWidth = selected ? 3 : 2
				context.beginPath()
				context.arc(point.x, point.y, radius, 0, Math.PI * 2)
				context.fill()
				context.stroke()
				context.fillStyle = '#FFFFFF'
				context.font = '700 9px system-ui'
				context.textAlign = 'center'
				context.textBaseline = 'middle'
				context.fillText(featureLabel(feature.kind).slice(0, 1).toUpperCase(), point.x, point.y)
			}
		}
		if (completed) drawCompletionBadge(context, point.x + 10, point.y - 10)
		context.restore()
	}
}

function drawOres(context: CanvasRenderingContext2D) {
	if (scale.value > SEED_MAP_ORE_MAX_SCALE) return
	for (const hit of oreHits.value) {
		const point = worldToScreen(hit.x, hit.z)
		if (
			point.x < -20 ||
			point.x > viewport.width + 20 ||
			point.y < -20 ||
			point.y > viewport.height + 20
		)
			continue
		const key = seedMapOreKey(hit)
		const selected = selectedOreKey.value === key
		const completed = workspace.completedOres.includes(key)
		const definition = seedMapOreDefinition(hit.ore)
		const variant = hit.y < 0 && definition.deepslateTexture ? 'deepslate' : 'stone'
		const image = oreImages.get(`${hit.ore}:${variant}`)
		const size = Math.max(10, Math.min(18, 1.2 / scale.value)) + (selected ? 4 : 0)
		context.save()
		context.globalAlpha = completed ? 0.42 : hit.verified ? 1 : 0.78
		context.imageSmoothingEnabled = false
		if (image) {
			context.drawImage(image, point.x - size / 2, point.y - size / 2, size, size)
		} else {
			context.fillStyle = '#B6B7BA'
			context.fillRect(point.x - size / 2, point.y - size / 2, size, size)
		}
		context.strokeStyle = selected ? '#FFFFFF' : 'rgba(15, 18, 20, 0.82)'
		context.lineWidth = selected ? 2.5 : 1.5
		context.strokeRect(point.x - size / 2, point.y - size / 2, size, size)
		if (completed) drawCompletionBadge(context, point.x + size / 2, point.y - size / 2)
		context.restore()
	}
}

function drawSpawn(context: CanvasRenderingContext2D) {
	if (spawn.value && workspace.dimension === 'overworld' && workspace.showSpawn) {
		const point = worldToScreen(spawn.value.x, spawn.value.z)
		context.save()
		if (spawnImage) {
			context.shadowColor = 'rgba(0, 0, 0, 0.48)'
			context.shadowBlur = 5
			context.shadowOffsetY = 2
			context.drawImage(spawnImage, point.x - 16, point.y - 16, 32, 32)
		} else {
			context.fillStyle = '#2E3138'
			context.strokeStyle = '#FFFFFF'
			context.lineWidth = 2
			context.beginPath()
			context.arc(point.x, point.y, 10, 0, Math.PI * 2)
			context.fill()
			context.stroke()
		}
		context.restore()
	}
}

function drawCompletionBadge(context: CanvasRenderingContext2D, x: number, y: number) {
	context.globalAlpha = 1
	context.shadowColor = 'transparent'
	context.fillStyle = '#1BD96A'
	context.strokeStyle = 'rgba(15, 18, 20, 0.9)'
	context.lineWidth = 2
	context.beginPath()
	context.arc(x, y, 7, 0, Math.PI * 2)
	context.fill()
	context.stroke()
	context.strokeStyle = '#FFFFFF'
	context.lineWidth = 1.75
	context.lineCap = 'round'
	context.lineJoin = 'round'
	context.beginPath()
	context.moveTo(x - 3, y)
	context.lineTo(x - 0.5, y + 2.5)
	context.lineTo(x + 3.5, y - 2.5)
	context.stroke()
}

function drawMarkers(context: CanvasRenderingContext2D) {
	for (const marker of workspace.markers) {
		const point = worldToScreen(marker.x, marker.z)
		if (
			point.x < -20 ||
			point.x > viewport.width + 20 ||
			point.y < -20 ||
			point.y > viewport.height + 20
		)
			continue
		context.save()
		context.fillStyle = marker.color
		context.strokeStyle = '#FFFFFF'
		context.lineWidth = 2
		context.beginPath()
		context.arc(point.x, point.y - 3, 7, 0, Math.PI * 2)
		context.moveTo(point.x - 4, point.y + 2)
		context.lineTo(point.x, point.y + 11)
		context.lineTo(point.x + 4, point.y + 2)
		context.fill()
		context.stroke()
		if (showLayerNames.value) {
			context.font = '600 11px system-ui'
			const width = context.measureText(marker.name).width + 10
			context.fillStyle = 'rgba(20, 22, 26, 0.86)'
			context.fillRect(point.x + 10, point.y - 12, width, 22)
			context.fillStyle = '#FFFFFF'
			context.textAlign = 'left'
			context.textBaseline = 'middle'
			context.fillText(marker.name, point.x + 15, point.y - 1)
		}
		context.restore()
	}
}

function drawRuler(context: CanvasRenderingContext2D) {
	if (!rulerPoints.value.length) return
	const points = rulerPoints.value.map((point) => worldToScreen(point.x, point.z))
	context.save()
	context.strokeStyle = '#F2B84B'
	context.fillStyle = '#F2B84B'
	context.lineWidth = 2
	context.setLineDash([6, 4])
	context.beginPath()
	context.arc(points[0].x, points[0].y, 4, 0, Math.PI * 2)
	context.fill()
	if (points[1]) {
		context.beginPath()
		context.moveTo(points[0].x, points[0].y)
		context.lineTo(points[1].x, points[1].y)
		context.stroke()
		context.setLineDash([])
		context.beginPath()
		context.arc(points[1].x, points[1].y, 4, 0, Math.PI * 2)
		context.fill()
	}
	context.restore()
}

function scheduleRefresh(delay = 80) {
	if (!viewport.width || !viewport.height) return
	requestEpoch++
	clearQueuedTileJobs()
	redraw()
	if (refreshTimer) window.clearTimeout(refreshTimer)
	refreshTimer = window.setTimeout(refreshMap, delay)
}

function refreshMap() {
	refreshTimer = undefined
	const epoch = requestEpoch
	if (!workspace.seed.trim()) {
		features.value = []
		spawn.value = null
		loading.value = false
		refreshCurrentOreLayer(0)
		redraw()
		return
	}
	if (!selectedProfile.value?.available) {
		features.value = []
		spawn.value = null
		refreshCurrentOreLayer(0)
		redraw()
		return
	}
	mapError.value = ''
	if (centerOnNextSpawn && workspace.dimension === 'overworld' && workspace.showSpawn) {
		void requestSpawn(epoch)
		return
	}
	requestVisibleTiles(epoch)
	if (workspace.displayMode === 'structures') void requestFeatures(epoch)
	else features.value = []
	void requestSpawn(epoch)
	refreshCurrentOreLayer()
}

function refreshCurrentOreLayer(delay = 180) {
	refreshOreLayer(
		{
			enabled: workspace.displayMode === 'ores',
			seed: workspace.seed,
			version: workspace.gameVersion,
			dimension: workspace.dimension,
			scale: scale.value,
			selectedOres: [...activeSelectedOres.value],
			yMin: workspace.oreYMin,
			yMax: workspace.oreYMax,
			bounds: mapBounds(),
			center: { ...workspace.center },
		},
		delay,
	)
}

function requestVisibleTiles(epoch: number) {
	const bounds = mapBounds()
	const requestScale = tileScale.value
	const tileSpan = 256 * requestScale
	const minTileX = Math.floor(bounds.minX / tileSpan)
	const minTileZ = Math.floor(bounds.minZ / tileSpan)
	const maxTileX = Math.floor(bounds.maxX / tileSpan)
	const maxTileZ = Math.floor(bounds.maxZ / tileSpan)
	const candidates: { tileX: number; tileZ: number }[] = []
	for (let tileX = minTileX; tileX <= maxTileX; tileX++) {
		for (let tileZ = minTileZ; tileZ <= maxTileZ; tileZ++) candidates.push({ tileX, tileZ })
	}
	candidates.sort(
		(a, b) =>
			Math.hypot(
				a.tileX * tileSpan + tileSpan / 2 - workspace.center.x,
				a.tileZ * tileSpan + tileSpan / 2 - workspace.center.z,
			) -
			Math.hypot(
				b.tileX * tileSpan + tileSpan / 2 - workspace.center.x,
				b.tileZ * tileSpan + tileSpan / 2 - workspace.center.z,
			),
	)

	let hasMissingTiles = false
	for (const { tileX, tileZ } of candidates) {
		const key = tileKey(tileX, tileZ, requestScale)
		if (tileImages.has(key) || pendingTiles.get(key) === epoch) continue
		hasMissingTiles = true
		const request: SeedMapTileRequest = {
			epoch,
			seed: workspace.seed,
			edition: workspace.edition,
			version: workspace.gameVersion,
			dimension: workspace.dimension,
			x: tileX * tileSpan,
			z: tileZ * tileSpan,
			scale: requestScale,
			width: 256,
			height: 256,
			elevation: workspace.elevation,
			terrain: terrainEnabled.value,
			contours: contoursEnabled.value,
			highlightBiomes:
				workspace.highlightBiomeEnabled && activeHighlightedBiomes.value.length > 0
					? activeHighlightedBiomes.value
					: undefined,
		}
		pendingTiles.set(key, epoch)
		enqueueTile({
			epoch,
			key,
			run: async () => {
				try {
					if (!isCurrentSeedMapEpoch(epoch, requestEpoch)) return
					const tile = await renderSeedMapTile(request)
					if (!isCurrentSeedMapEpoch(tile.epoch, requestEpoch)) {
						tile.bitmap.close()
						return
					}
					tileImages.get(key)?.close()
					tileImages.set(key, tile.bitmap)
					while (tileImages.size > 96) {
						const oldest = tileImages.keys().next().value
						if (typeof oldest !== 'string') break
						tileImages.get(oldest)?.close()
						tileImages.delete(oldest)
					}
					redraw()
				} catch (error) {
					if (isCurrentSeedMapEpoch(epoch, requestEpoch)) {
						mapError.value = error instanceof Error ? error.message : String(error)
					}
				} finally {
					if (pendingTiles.get(key) === epoch) pendingTiles.delete(key)
				}
			},
		})
	}
	if (!hasMissingTiles) fallbackTileScale = undefined
}

function enqueueTile(job: TileJob) {
	tileQueue.push(job)
	pumpTileQueue()
}

function pumpTileQueue() {
	const concurrency = seedMapTileConcurrency(tileScale.value)
	const totalConcurrency = concurrency * 2
	while (
		(activeTileRequestsByEpoch.get(requestEpoch) ?? 0) < concurrency &&
		activeTileRequests < totalConcurrency &&
		tileQueue.length
	) {
		const job = tileQueue.shift()
		if (!job) return
		activeTileRequests += 1
		activeTileRequestsByEpoch.set(job.epoch, (activeTileRequestsByEpoch.get(job.epoch) ?? 0) + 1)
		loading.value = true
		void job.run().finally(() => {
			activeTileRequests -= 1
			const epochRequests = (activeTileRequestsByEpoch.get(job.epoch) ?? 1) - 1
			if (epochRequests > 0) activeTileRequestsByEpoch.set(job.epoch, epochRequests)
			else activeTileRequestsByEpoch.delete(job.epoch)
			loading.value =
				(activeTileRequestsByEpoch.get(requestEpoch) ?? 0) > 0 ||
				tileQueue.some((queuedJob) => queuedJob.epoch === requestEpoch)
			if (!loading.value && job.epoch === requestEpoch) {
				fallbackTileScale = undefined
				redraw()
			}
			pumpTileQueue()
		})
	}
}

function clearQueuedTileJobs() {
	const queuedJobs = tileQueue.splice(0, tileQueue.length)
	for (const job of queuedJobs) {
		if (pendingTiles.get(job.key) === job.epoch) pendingTiles.delete(job.key)
	}
}

function clearTileImages() {
	for (const bitmap of tileImages.values()) bitmap.close()
	tileImages.clear()
}

async function requestFeatures(epoch: number) {
	if (activeFeatureMask.value === 0) {
		features.value = []
		redraw()
		return
	}
	const bounds = mapBounds()
	try {
		const result = await findSeedMapFeatures({
			seed: workspace.seed,
			edition: workspace.edition,
			version: workspace.gameVersion,
			dimension: workspace.dimension,
			minX: bounds.minX,
			minZ: bounds.minZ,
			maxX: bounds.maxX,
			maxZ: bounds.maxZ,
			featureMask: activeFeatureMask.value,
		})
		if (epoch !== requestEpoch) return
		features.value = result
		redraw()
	} catch (error) {
		if (epoch === requestEpoch)
			mapError.value = error instanceof Error ? error.message : String(error)
	}
}

async function requestSpawn(epoch: number) {
	if (workspace.dimension !== 'overworld' || !workspace.showSpawn) {
		spawn.value = null
		return
	}
	const contextKey = `${workspace.seed}|${workspace.edition}|${workspace.gameVersion}`
	if (spawn.value && spawnContextKey === contextKey) {
		redraw()
		return
	}
	spawn.value = null
	try {
		const result = await getSeedMapSpawn(workspace.seed, workspace.edition, workspace.gameVersion)
		if (epoch !== requestEpoch) return
		spawn.value = result
		spawnContextKey = contextKey
		if (centerOnNextSpawn) {
			centerOnNextSpawn = false
			workspace.center = { x: result.x, z: result.z }
			selection.value = { x: result.x, z: result.z, spawn: true }
			scheduleRefresh()
			return
		}
		redraw()
	} catch {
		if (epoch === requestEpoch) {
			spawn.value = null
			spawnContextKey = ''
			if (centerOnNextSpawn) {
				centerOnNextSpawn = false
				scheduleRefresh()
			}
		}
	}
}

function onPointerDown(event: PointerEvent) {
	if (!canvas.value || event.button !== 0) return
	clearWheelGesture()
	stopZoomAnimation()
	canvas.value.setPointerCapture(event.pointerId)
	activePointers.set(event.pointerId, { x: event.offsetX, y: event.offsetY })
	if (activePointers.size === 2) {
		flushPendingPan()
		pinchDistance = currentPinchDistance()
		suppressPointerSelection = true
		clearLongPress()
		return
	}
	pointer.id = event.pointerId
	pointer.startX = event.offsetX
	pointer.startY = event.offsetY
	pointer.centerX = workspace.center.x
	pointer.centerZ = workspace.center.z
	pointer.moved = false
	suppressPointerSelection = false
	if (event.pointerType === 'touch') {
		clearLongPress()
		longPressTimer = window.setTimeout(() => {
			if (pointer.id !== event.pointerId || pointer.moved) return
			openMarkerDraftAt(event.offsetX, event.offsetY)
			suppressPointerSelection = true
		}, 650)
	}
}

function onPointerMove(event: PointerEvent) {
	if (!activePointers.has(event.pointerId)) return
	activePointers.set(event.pointerId, { x: event.offsetX, y: event.offsetY })
	if (activePointers.size === 2) {
		const distance = currentPinchDistance()
		if (pinchDistance > 0 && (distance > pinchDistance * 1.45 || distance < pinchDistance * 0.69)) {
			const points = [...activePointers.values()]
			zoomAt(
				(points[0].x + points[1].x) / 2,
				(points[0].y + points[1].y) / 2,
				distance > pinchDistance ? -0.35 : 0.35,
			)
			pinchDistance = distance
		}
		return
	}
	if (pointer.id !== event.pointerId) return
	const deltaX = event.offsetX - pointer.startX
	const deltaY = event.offsetY - pointer.startY
	if (Math.abs(deltaX) > 3 || Math.abs(deltaY) > 3) {
		pointer.moved = true
		clearLongPress()
	}
	if (!pointer.moved) return
	pendingPanCenter = {
		x: clampWorldCoordinate(pointer.centerX - deltaX * scale.value),
		z: clampWorldCoordinate(pointer.centerZ - deltaY * scale.value),
	}
	if (panFrame === undefined) panFrame = window.requestAnimationFrame(flushPendingPan)
}

function onPointerUp(event: PointerEvent) {
	if (!activePointers.has(event.pointerId)) return
	activePointers.delete(event.pointerId)
	clearLongPress()
	canvas.value?.releasePointerCapture(event.pointerId)
	if (pointer.id !== event.pointerId) return
	flushPendingPan()
	const clicked = !pointer.moved && !suppressPointerSelection
	pointer.id = -1
	if (clicked) selectAt(event.offsetX, event.offsetY)
	else scheduleRefresh(40)
	if (activePointers.size < 2) pinchDistance = 0
}

function flushPendingPan() {
	if (panFrame !== undefined) window.cancelAnimationFrame(panFrame)
	panFrame = undefined
	if (!pendingPanCenter) return
	workspace.center.x = pendingPanCenter.x
	workspace.center.z = pendingPanCenter.z
	pendingPanCenter = undefined
	redraw()
}

function onWheel(event: WheelEvent) {
	event.preventDefault()
	const now = performance.now()
	const deltaMultiplier =
		event.deltaMode === WheelEvent.DOM_DELTA_LINE
			? 40
			: event.deltaMode === WheelEvent.DOM_DELTA_PAGE
				? 300
				: 1
	const normalizedDelta = event.deltaY * deltaMultiplier
	if (normalizedDelta === 0) return
	if (!wheelMode || now - wheelGestureStartedAt > 400) {
		clearWheelGesture()
		wheelMode = Math.abs(normalizedDelta) < 4 ? 'trackpad' : 'wheel'
		wheelGestureStartedAt = now
		if (wheelMode === 'trackpad') stopZoomAnimation()
	}
	wheelAnchor = { x: event.offsetX, y: event.offsetY }
	if (wheelMode === 'trackpad') {
		wheelGestureStartedAt = now
		applyZoomAt(event.offsetX, event.offsetY, normalizedDelta / 600)
		if (trackpadTimer) window.clearTimeout(trackpadTimer)
		trackpadTimer = window.setTimeout(() => {
			trackpadTimer = undefined
			scheduleRefresh(60)
		}, 80)
		return
	}

	wheelDelta += normalizedDelta
	if (wheelTimer) window.clearTimeout(wheelTimer)
	const timeLeft = Math.max(80 - (now - wheelGestureStartedAt), 0)
	wheelTimer = window.setTimeout(applyWheelZoom, timeLeft)
}

function applyWheelZoom() {
	wheelTimer = undefined
	const deltaZoom = Math.min(Math.max(wheelDelta, -300), 300) / 600
	const anchor = wheelAnchor
	clearWheelGesture()
	if (deltaZoom === 0) return
	animateZoomTo(
		anchor.x,
		anchor.y,
		Math.min(Math.max(workspace.zoom + deltaZoom, SEED_MAP_MIN_ZOOM), SEED_MAP_SCALES.length - 1),
		250,
	)
}

function clearWheelGesture() {
	if (wheelTimer) window.clearTimeout(wheelTimer)
	if (trackpadTimer) window.clearTimeout(trackpadTimer)
	wheelTimer = undefined
	trackpadTimer = undefined
	wheelMode = undefined
	wheelGestureStartedAt = 0
	wheelDelta = 0
}

function onDoubleClick(event: MouseEvent) {
	event.preventDefault()
	zoomAt(event.offsetX, event.offsetY, -0.5)
}

function onContextMenu(event: MouseEvent) {
	event.preventDefault()
	openMarkerDraftAt(event.offsetX, event.offsetY)
}

function onMapKeydown(event: KeyboardEvent) {
	const target = event.target
	if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return
	let mapMoved = false
	const panDistance = Math.max(
		64,
		Math.round(Math.min(viewport.width, viewport.height) * scale.value * 0.18),
	)
	if (event.key === 'ArrowLeft') {
		workspace.center.x = clampWorldCoordinate(workspace.center.x - panDistance)
		mapMoved = true
	} else if (event.key === 'ArrowRight') {
		workspace.center.x = clampWorldCoordinate(workspace.center.x + panDistance)
		mapMoved = true
	} else if (event.key === 'ArrowUp') {
		workspace.center.z = clampWorldCoordinate(workspace.center.z - panDistance)
		mapMoved = true
	} else if (event.key === 'ArrowDown') {
		workspace.center.z = clampWorldCoordinate(workspace.center.z + panDistance)
		mapMoved = true
	} else if (event.key === '+' || event.key === '=') zoomBy(-0.5)
	else if (event.key === '-') zoomBy(0.5)
	else if (event.key.toLowerCase() === 'f') void toggleFullscreen()
	else if (event.key === 'Escape') {
		selection.value = null
		advancedOpen.value = false
		layersExpanded.value = false
		if (isFullscreen.value) void document.exitFullscreen()
		else return
	} else return
	event.preventDefault()
	if (mapMoved) scheduleRefresh()
}

function currentPinchDistance() {
	const points = [...activePointers.values()]
	if (points.length !== 2) return 0
	return Math.hypot(points[1].x - points[0].x, points[1].y - points[0].y)
}

function clearLongPress() {
	if (!longPressTimer) return
	window.clearTimeout(longPressTimer)
	longPressTimer = undefined
}

function selectAt(x: number, y: number) {
	const point = screenToWorld(x, y)
	const marker = workspace.markers.find((item) => {
		const screen = worldToScreen(item.x, item.z)
		return Math.hypot(screen.x - x, screen.y - y) <= 13
	})
	const feature =
		workspace.displayMode === 'structures'
			? displayedFeatures.value
					.map((item) => ({ item, point: worldToScreen(item.x, item.z) }))
					.find((candidate) => Math.hypot(candidate.point.x - x, candidate.point.y - y) <= 13)?.item
			: undefined
	const ore =
		workspace.displayMode === 'ores' && scale.value <= SEED_MAP_ORE_MAX_SCALE
			? oreHits.value
					.map((item) => ({ item, point: worldToScreen(item.x, item.z) }))
					.filter((candidate) => Math.hypot(candidate.point.x - x, candidate.point.y - y) <= 12)
					.sort(
						(a, b) =>
							Math.hypot(a.point.x - x, a.point.y - y) - Math.hypot(b.point.x - x, b.point.y - y),
					)[0]?.item
			: undefined
	const spawnSelected =
		workspace.showSpawn &&
		spawn.value !== null &&
		Math.hypot(
			worldToScreen(spawn.value.x, spawn.value.z).x - x,
			worldToScreen(spawn.value.x, spawn.value.z).y - y,
		) <= 14
	selection.value = marker
		? { x: marker.x, z: marker.z, markerId: marker.id }
		: feature
			? { x: feature.x, z: feature.z, feature }
			: ore
				? { x: ore.x, z: ore.z, ore }
				: spawnSelected && spawn.value
					? { x: spawn.value.x, z: spawn.value.z, spawn: true }
					: point
	if (!marker && !feature && !ore && !(spawnSelected && spawn.value)) {
		void lookupSelectionBiome(point)
	}
	markerDraftOpen.value = false
	if (rulerEnabled.value) {
		rulerPoints.value = rulerPoints.value.length === 1 ? [rulerPoints.value[0], point] : [point]
	}
	redraw()
}

async function lookupSelectionBiome(point: { x: number; z: number }) {
	if (!workspace.seed.trim() || !selectedProfile.value?.available) return
	const token = ++biomeLookupToken
	try {
		const biome = await getSeedMapBiomeAt({
			seed: workspace.seed,
			edition: workspace.edition,
			version: workspace.gameVersion,
			dimension: workspace.dimension,
			x: point.x,
			y: workspace.elevation,
			z: point.z,
		})
		if (token !== biomeLookupToken || biome < 0) return
		const current = selection.value
		if (
			!current ||
			current.x !== point.x ||
			current.z !== point.z ||
			current.feature ||
			current.ore ||
			current.markerId ||
			current.spawn
		) {
			return
		}
		selection.value = { ...current, biome }
	} catch {
		// The plain location popup stays when the engine cannot resolve a biome.
	}
}

function biomeDisplayName(biome: number) {
	const name = SEED_MAP_BIOME_NAMES[biome]
	if (!name) return `#${biome}`
	return formatMessage({
		id: `app.lab.seed-map.biome.${seedMapBiomeSlug(name)}`,
		defaultMessage: name,
	})
}

function biomeColorOf(biome: number) {
	return SEED_MAP_BIOMES.find((item) => item.id === biome)?.color ?? 'transparent'
}

function openMarkerDraftAt(x: number, y: number) {
	const point = screenToWorld(x, y)
	selection.value = point
	markerName.value = ''
	markerDraftOpen.value = true
	redraw()
}

function zoomAt(x: number, y: number, delta: number) {
	clearWheelGesture()
	const nextZoom = Math.min(
		Math.max(workspace.zoom + delta, SEED_MAP_MIN_ZOOM),
		SEED_MAP_SCALES.length - 1,
	)
	animateZoomTo(x, y, nextZoom, 250)
}

function zoomBy(delta: number) {
	zoomAt(viewport.width / 2, viewport.height / 2, delta)
}

function applyZoomAt(x: number, y: number, delta: number) {
	const targetZoom = Math.min(
		Math.max(workspace.zoom + delta, SEED_MAP_MIN_ZOOM),
		SEED_MAP_SCALES.length - 1,
	)
	if (Math.abs(targetZoom - workspace.zoom) < 0.0001) return
	const anchor = {
		x: workspace.center.x + (x - viewport.width / 2) * scale.value,
		z: workspace.center.z + (y - viewport.height / 2) * scale.value,
	}
	const previousTileScale = tileScale.value
	workspace.zoom = targetZoom
	if (tileScale.value !== previousTileScale) fallbackTileScale = previousTileScale
	workspace.center.x = clampWorldCoordinate(anchor.x - (x - viewport.width / 2) * scale.value)
	workspace.center.z = clampWorldCoordinate(anchor.z - (y - viewport.height / 2) * scale.value)
	redraw()
}

function animateZoomTo(x: number, y: number, targetZoom: number, duration: number) {
	if (Math.abs(targetZoom - workspace.zoom) < 0.0001) return
	if (zoomAnimationFrame !== undefined) window.cancelAnimationFrame(zoomAnimationFrame)
	const startTime = performance.now()
	const startZoom = workspace.zoom
	const anchor = {
		x: workspace.center.x + (x - viewport.width / 2) * scale.value,
		z: workspace.center.z + (y - viewport.height / 2) * scale.value,
	}
	const step = (now: number) => {
		const progress = Math.min(1, (now - startTime) / duration)
		const easedProgress = 1 - (1 - progress) ** 3
		const previousTileScale = tileScale.value
		workspace.zoom = startZoom + (targetZoom - startZoom) * easedProgress
		if (tileScale.value !== previousTileScale) fallbackTileScale = previousTileScale
		workspace.center.x = clampWorldCoordinate(anchor.x - (x - viewport.width / 2) * scale.value)
		workspace.center.z = clampWorldCoordinate(anchor.z - (y - viewport.height / 2) * scale.value)
		redraw()
		if (progress < 1) {
			zoomAnimationFrame = window.requestAnimationFrame(step)
			return
		}
		zoomAnimationFrame = undefined
		scheduleRefresh(60)
	}
	zoomAnimationFrame = window.requestAnimationFrame(step)
}

function stopZoomAnimation() {
	if (zoomAnimationFrame !== undefined) window.cancelAnimationFrame(zoomAnimationFrame)
	zoomAnimationFrame = undefined
}

function toggleFeature(feature: SeedMapFeatureKind) {
	workspace.visibleFeatures = workspace.visibleFeatures.includes(feature)
		? workspace.visibleFeatures.filter((kind) => kind !== feature)
		: [...workspace.visibleFeatures, feature]
}

function toggleOre(ore: SeedMapOreKind) {
	workspace.selectedOres = workspace.selectedOres.includes(ore)
		? workspace.selectedOres.filter((kind) => kind !== ore)
		: [...workspace.selectedOres, ore]
}

function selectAllOres() {
	workspace.selectedOres = [
		...new Set([...workspace.selectedOres, ...dimensionOres.value.map((ore) => ore.kind)]),
	]
}

function clearOres() {
	const current = new Set(dimensionOres.value.map((ore) => ore.kind))
	workspace.selectedOres = workspace.selectedOres.filter((ore) => !current.has(ore))
}

function resetOres() {
	const current = new Set(dimensionOres.value.map((ore) => ore.kind))
	const defaultOre = workspace.dimension === 'nether' ? 'netherite' : 'diamond'
	workspace.selectedOres = [
		...workspace.selectedOres.filter((ore) => !current.has(ore)),
		...(current.has(defaultOre) ? [defaultOre] : []),
	]
	resetOreYRange()
}

function resetOreYRange() {
	workspace.oreYMin = null
	workspace.oreYMax = null
}

function selectAllFeatures() {
	const selected = new Set(workspace.visibleFeatures)
	for (const feature of dimensionFeatures.value) selected.add(feature.kind)
	workspace.visibleFeatures = [...selected]
}

function clearFeatures() {
	const current = new Set(dimensionFeatures.value.map((feature) => feature.kind))
	workspace.visibleFeatures = workspace.visibleFeatures.filter((feature) => !current.has(feature))
	workspace.showSpawn = false
}

function resetFeatures() {
	const defaults = createDefaultSeedMapWorkspace().visibleFeatures
	const current = new Set(dimensionFeatures.value.map((feature) => feature.kind))
	workspace.visibleFeatures = [
		...workspace.visibleFeatures.filter((feature) => !current.has(feature)),
		...defaults.filter((feature) => current.has(feature)),
	]
	workspace.showSpawn = true
}

function randomizeSeed() {
	const bytes = new Uint32Array(2)
	crypto.getRandomValues(bytes)
	const value = (BigInt(bytes[0]) << 32n) | BigInt(bytes[1])
	pendingHistorySource = 'random'
	workspace.seed = BigInt.asIntN(64, value).toString()
}

function onWorldImported(selection: SeedMapWorldImport) {
	workspace.edition = 'java'
	if (
		selection.version &&
		profiles.value.some(
			(profile) => profile.edition === 'java' && profile.version === selection.version,
		)
	) {
		workspace.gameVersion = selection.version
	}
	pendingHistorySource = 'instance'
	pendingHistoryLabels = {
		instanceName: selection.instance.name,
		worldName: selection.world.name,
	}
	if (workspace.seed === selection.seed) {
		recordHistory('instance', pendingHistoryLabels)
		pendingHistorySource = null
		pendingHistoryLabels = {}
	} else {
		workspace.seed = selection.seed
	}
	addNotification({
		type: 'success',
		title: formatMessage(messages.worldSeedLoaded, { world: selection.world.name }),
	})
}

function recordHistory(
	source: SeedMapHistorySource,
	labels: { instanceName?: string; worldName?: string } = {},
) {
	if (!workspace.seed.trim()) return
	historyEntries.value = recordSeedMapHistory({
		seed: workspace.seed,
		edition: workspace.edition,
		gameVersion: workspace.gameVersion,
		source,
		completedFeatures: [...workspace.completedFeatures],
		completedOres: [...workspace.completedOres],
		...labels,
	})
}

function stashActiveProgress() {
	historyEntries.value = updateSeedMapHistoryProgress(
		activeHistoryId,
		[...workspace.completedFeatures],
		[...workspace.completedOres],
	)
}

function applyHistoryEntry(entry: SeedMapHistoryEntry) {
	pendingHistorySource = entry.source
	pendingHistoryLabels = { instanceName: entry.instanceName, worldName: entry.worldName }
	workspace.edition = entry.edition
	if (
		profiles.value.some(
			(profile) => profile.edition === entry.edition && profile.version === entry.gameVersion,
		)
	) {
		workspace.gameVersion = entry.gameVersion
	}
	if (workspace.seed === entry.seed) {
		if (activeHistoryId !== entry.id) {
			stashActiveProgress()
			activeHistoryId = entry.id
		}
		workspace.completedFeatures = [...entry.completedFeatures]
		workspace.completedOres = [...entry.completedOres]
		recordHistory(entry.source, pendingHistoryLabels)
		pendingHistorySource = null
		pendingHistoryLabels = {}
	} else {
		workspace.seed = entry.seed
	}
}

function removeHistory(id: string) {
	historyEntries.value = removeSeedMapHistoryEntry(id)
}

function clearHistory() {
	historyEntries.value = clearSeedMapHistory()
}

function historySourceLabel(entry: SeedMapHistoryEntry) {
	if (entry.source === 'instance') {
		const parts = [entry.instanceName, entry.worldName].filter(Boolean)
		if (parts.length > 0) return parts.join(' · ')
	}
	if (entry.source === 'random') return formatMessage(messages.historySourceRandom)
	if (entry.source === 'share') return formatMessage(messages.historySourceShare)
	return formatMessage(messages.historySourceManual)
}

function goToCoordinates() {
	const x = Number(coordinateX.value)
	const z = Number(coordinateZ.value)
	if (!Number.isFinite(x) || !Number.isFinite(z)) return
	workspace.center.x = clampWorldCoordinate(x)
	workspace.center.z = clampWorldCoordinate(z)
	selection.value = { x: Math.round(workspace.center.x), z: Math.round(workspace.center.z) }
	scheduleRefresh()
}

function centerSpawn() {
	if (!spawn.value) return
	workspace.center.x = spawn.value.x
	workspace.center.z = spawn.value.z
	selection.value = { x: spawn.value.x, z: spawn.value.z, spawn: true }
	scheduleRefresh()
}

function addMarker() {
	if (!selection.value) return
	const marker: SeedMapMarker = {
		id: crypto.randomUUID(),
		name: markerName.value.trim() || `${selection.value.x}, ${selection.value.z}`,
		x: selection.value.x,
		z: selection.value.z,
		color: markerColor.value,
	}
	workspace.markers.push(marker)
	selection.value = { x: marker.x, z: marker.z, markerId: marker.id }
	markerName.value = ''
	markerDraftOpen.value = false
	addNotification({ type: 'success', title: formatMessage(messages.markerSaved) })
}

function removeMarker(id: string) {
	workspace.markers = workspace.markers.filter((marker) => marker.id !== id)
	if (selection.value?.markerId === id) selection.value = null
}

function jumpToMarker(marker: SeedMapMarker) {
	workspace.center = { x: marker.x, z: marker.z }
	selection.value = { x: marker.x, z: marker.z, markerId: marker.id }
	advancedOpen.value = false
	scheduleRefresh()
}

async function copyTeleport() {
	if (!selection.value) return
	try {
		const y = selection.value.ore?.y ?? '~'
		await navigator.clipboard.writeText(`/tp @s ${selection.value.x} ${y} ${selection.value.z}`)
		addNotification({ type: 'success', title: formatMessage(messages.teleportCopied) })
	} catch (error) {
		handleError(error)
	}
}

async function copyShareLink() {
	try {
		const query = new URLSearchParams(createShareQuery(workspace))
		await navigator.clipboard.writeText(`axolotl://seed-map?${query.toString()}`)
		addNotification({ type: 'success', title: formatMessage(messages.shareCopied) })
	} catch (error) {
		handleError(error)
	}
}

function focusNearestFeature() {
	if (!displayedFeatures.value.length) return
	const nearest = [...displayedFeatures.value].sort(
		(a, b) =>
			Math.hypot(a.x - workspace.center.x, a.z - workspace.center.z) -
			Math.hypot(b.x - workspace.center.x, b.z - workspace.center.z),
	)[0]
	selection.value = { x: nearest.x, z: nearest.z, feature: nearest }
	workspace.center.x = nearest.x
	workspace.center.z = nearest.z
	scheduleRefresh()
}

async function toggleFullscreen() {
	try {
		if (document.fullscreenElement) await document.exitFullscreen()
		else await fullscreenContainer.value?.requestFullscreen()
	} catch (error) {
		handleError(error)
	}
}

function onFullscreenChange() {
	isFullscreen.value = document.fullscreenElement === fullscreenContainer.value
	nextTick(resizeCanvas)
}

function clampWorldCoordinate(value: number) {
	return Math.round(Math.min(Math.max(value, -30_000_000), 30_000_000))
}
</script>

<template>
	<main class="seed-map-page mx-auto flex h-full min-h-0 w-full max-w-[110rem] flex-col gap-3 p-4">
		<header class="flex min-h-9 items-center justify-between gap-3">
			<div class="min-w-0">
				<h1 class="m-0 truncate text-xl font-bold text-contrast">
					{{ formatMessage(messages.title) }}
				</h1>
				<p class="m-0 truncate text-xs text-secondary">
					{{ editionLabel(workspace.edition) }} {{ workspace.gameVersion }} ·
					{{ dimensionLabel(workspace.dimension) }}
				</p>
			</div>
			<div class="seed-map-header-actions flex shrink-0 items-center gap-2">
				<span class="map-scale-badge">{{
					formatMessage(messages.mapScale, { scale: mapScaleLabel })
				}}</span>
				<ButtonStyled circular type="outlined">
					<button
						v-tooltip="formatMessage(messages.copyright)"
						:aria-label="formatMessage(messages.copyright)"
						@click="copyrightModal?.show($event)"
					>
						<CircleAlertIcon />
					</button>
				</ButtonStyled>
			</div>
		</header>

		<section data-onboarding-id="seed-map-toolbar" class="seed-map-toolbar">
			<div class="toolbar-primary">
				<div class="control-group seed-map-edition">
					<span class="control-label">{{ formatMessage(messages.edition) }}</span>
					<div class="seed-map-dropdown min-w-0">
						<DropdownSelect
							v-model="workspace.edition"
							:options="editionOptions"
							:display-name="editionLabel"
							name="Seed map edition"
						/>
					</div>
				</div>
				<div class="control-group seed-map-version">
					<span class="control-label">{{ formatMessage(messages.gameVersion) }}</span>
					<div class="seed-map-dropdown min-w-0">
						<DropdownSelect
							v-model="workspace.gameVersion"
							:options="availableVersions"
							name="Seed map version"
						/>
					</div>
				</div>
				<div class="control-group seed-control">
					<span class="control-label">{{ formatMessage(messages.seed) }}</span>
					<div class="seed-input-row">
						<StyledInput
							v-model="workspace.seed"
							:icon="HashIcon"
							:placeholder="formatMessage(messages.seedPlaceholder)"
							wrapper-class="seed-map-seed w-full"
						/>
						<ButtonStyled circular type="outlined">
							<button
								v-tooltip="formatMessage(messages.randomSeed)"
								:aria-label="formatMessage(messages.randomSeed)"
								@click="randomizeSeed"
							>
								<RefreshCwIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled circular type="outlined">
							<PopoutMenu
								:aria-label="formatMessage(messages.history)"
								dropdown-class="seed-map-history-popout"
								placement="bottom-end"
							>
								<HistoryIcon />
								<template #menu>
									<div class="seed-history-menu">
										<div class="seed-history-heading">
											<strong>{{ formatMessage(messages.history) }}</strong>
											<ButtonStyled size="small" type="transparent">
												<button :disabled="historyEntries.length === 0" @click="clearHistory">
													{{ formatMessage(messages.clearHistory) }}
												</button>
											</ButtonStyled>
										</div>
										<p v-if="historyEntries.length === 0" class="seed-history-empty">
											{{ formatMessage(messages.historyEmpty) }}
										</p>
										<div v-else class="seed-history-list">
											<div v-for="entry in historyEntries" :key="entry.id" class="seed-history-row">
												<button class="seed-history-load" @click="applyHistoryEntry(entry)">
													<span class="seed-history-seed">{{ entry.seed }}</span>
													<span class="seed-history-meta">
														<template v-if="entry.gameVersion">{{ entry.gameVersion }} · </template>
														<template v-if="entry.edition === 'java-large-biomes'"
															>{{ formatMessage(messages.javaLargeBiomes) }} · </template
														>{{ historySourceLabel(entry) }} ·
														{{ formatRelativeTime(new Date(entry.lastViewedAt).toISOString())
														}}<span
															v-if="entry.completedFeatures.length + entry.completedOres.length > 0"
															class="seed-history-progress"
															>✓
															{{
																entry.completedFeatures.length + entry.completedOres.length
															}}</span
														>
													</span>
												</button>
												<ButtonStyled circular size="small" type="transparent">
													<button
														v-tooltip="formatMessage(messages.removeHistoryEntry)"
														class="seed-history-remove"
														:aria-label="formatMessage(messages.removeHistoryEntry)"
														@click="removeHistory(entry.id)"
													>
														<TrashIcon />
													</button>
												</ButtonStyled>
											</div>
										</div>
									</div>
								</template>
							</PopoutMenu>
						</ButtonStyled>
						<ButtonStyled type="outlined">
							<button
								v-tooltip="formatMessage(messages.importFromInstance)"
								:aria-label="formatMessage(messages.importFromInstance)"
								@click="worldImportModal?.show()"
							>
								<ImportIcon />{{ formatMessage(messages.importFromInstance) }}
							</button>
						</ButtonStyled>
					</div>
				</div>
			</div>

			<div class="toolbar-secondary">
				<div class="control-group dimension-group shrink-0">
					<span class="control-label">{{ formatMessage(messages.dimension) }}</span>
					<div
						class="dimension-control flex h-10 items-stretch overflow-hidden rounded-[var(--radius-md)] border border-surface-5 bg-surface-3"
						role="group"
						:aria-label="formatMessage(messages.dimension)"
					>
						<ButtonStyled
							v-for="dimension in dimensions"
							:key="dimension"
							size="small"
							:type="workspace.dimension === dimension ? 'standard' : 'transparent'"
							:color="workspace.dimension === dimension ? 'brand' : 'standard'"
						>
							<button
								class="dimension-option"
								:aria-pressed="workspace.dimension === dimension"
								@click="workspace.dimension = dimension"
							>
								{{ dimensionLabel(dimension) }}
							</button>
						</ButtonStyled>
					</div>
				</div>
				<div class="control-group coordinate-group min-w-80">
					<span class="control-label">{{ formatMessage(messages.coordinates) }}</span>
					<div class="coordinate-control" :aria-label="formatMessage(messages.coordinates)">
						<label class="coordinate-field relative flex min-w-0 items-center"
							><span>X</span
							><StyledInput v-model="coordinateX" type="number" @keydown.enter="goToCoordinates"
						/></label>
						<label class="coordinate-field relative flex min-w-0 items-center"
							><span>Z</span
							><StyledInput v-model="coordinateZ" type="number" @keydown.enter="goToCoordinates"
						/></label>
						<ButtonStyled color="brand"
							><button class="coordinate-go h-10" @click="goToCoordinates">
								<CompassIcon />{{ formatMessage(messages.go) }}
							</button></ButtonStyled
						>
					</div>
				</div>
				<div class="share-button">
					<ButtonStyled type="outlined">
						<button @click="copyShareLink"><ShareIcon />{{ formatMessage(messages.share) }}</button>
					</ButtonStyled>
				</div>
			</div>
		</section>

		<Admonition
			v-if="mapError"
			class="seed-map-error p-3"
			type="critical"
			:header="formatMessage(messages.mapFailed)"
			:body="mapError"
		/>

		<section
			ref="fullscreenContainer"
			class="seed-map-experience min-h-0 flex-1 bg-surface-1"
			data-onboarding-id="seed-map-workspace"
			tabindex="0"
			@keydown="onMapKeydown"
		>
			<div class="layer-strip border border-surface-5 bg-surface-2">
				<div class="layer-strip-heading">
					<span>{{
						formatMessage(
							workspace.displayMode === 'structures' ? messages.layers : messages.oreLayers,
						)
					}}</span>
					<div class="layer-mode-switch" role="group" :aria-label="formatMessage(messages.layers)">
						<ButtonStyled
							size="small"
							:type="workspace.displayMode === 'structures' ? 'standard' : 'transparent'"
							:color="workspace.displayMode === 'structures' ? 'brand' : 'standard'"
						>
							<button
								:aria-pressed="workspace.displayMode === 'structures'"
								@click="setDisplayMode('structures')"
							>
								<LandmarkIcon />{{ formatMessage(messages.structures) }}
							</button>
						</ButtonStyled>
						<ButtonStyled
							size="small"
							:type="workspace.displayMode === 'ores' ? 'standard' : 'transparent'"
							:color="workspace.displayMode === 'ores' ? 'brand' : 'standard'"
						>
							<button
								v-tooltip="oreModeSupported ? undefined : formatMessage(messages.oresRequireModern)"
								:aria-pressed="workspace.displayMode === 'ores'"
								:disabled="!oreModeSupported"
								@click="setDisplayMode('ores')"
							>
								<PickaxeIcon />{{ formatMessage(messages.ores) }}
							</button>
						</ButtonStyled>
					</div>
					<div class="flex items-center gap-1">
						<ButtonStyled circular type="standard"
							><button
								v-tooltip="formatMessage(showLayerNames ? messages.hideNames : messages.showNames)"
								:aria-label="
									formatMessage(showLayerNames ? messages.hideNames : messages.showNames)
								"
								@click="showLayerNames = !showLayerNames"
							>
								<EyeOffIcon v-if="showLayerNames" /><EyeIcon v-else /></button
						></ButtonStyled>
						<ButtonStyled circular type="standard"
							><button
								v-tooltip="
									formatMessage(layersExpanded ? messages.collapseLayers : messages.expandLayers)
								"
								:aria-label="
									formatMessage(layersExpanded ? messages.collapseLayers : messages.expandLayers)
								"
								@click="layersExpanded = !layersExpanded"
							>
								<ChevronDownIcon :class="{ 'rotate-180': layersExpanded }" /></button
						></ButtonStyled>
					</div>
				</div>
				<div class="layer-options">
					<template v-if="workspace.displayMode === 'structures'">
						<ButtonStyled
							v-for="feature in dimensionFeatures"
							:key="feature.kind"
							type="standard"
							:color="workspace.visibleFeatures.includes(feature.kind) ? 'brand' : 'standard'"
						>
							<button
								v-tooltip="featureTooltip(feature.kind, feature.maxScale)"
								class="layer-option w-10 p-1"
								:class="{
									'opacity-[0.68]': tileScale > feature.maxScale,
									named: showLayerNames,
								}"
								:aria-pressed="workspace.visibleFeatures.includes(feature.kind)"
								@click="toggleFeature(feature.kind)"
							>
								<img
									v-if="featureImageSource(feature.kind)"
									:src="featureImageSource(feature.kind)"
									alt=""
								/><component :is="featureIcons[feature.kind]" v-else /><span
									v-if="showLayerNames"
									>{{ featureLabel(feature.kind) }}</span
								>
							</button>
						</ButtonStyled>
					</template>
					<template v-else>
						<ButtonStyled
							v-for="ore in dimensionOres"
							:key="ore.kind"
							type="standard"
							:color="workspace.selectedOres.includes(ore.kind) ? 'brand' : 'standard'"
						>
							<button
								v-tooltip="oreLabel(ore.kind)"
								class="layer-option w-10 p-1"
								:class="{ named: showLayerNames }"
								:aria-pressed="workspace.selectedOres.includes(ore.kind)"
								@click="toggleOre(ore.kind)"
							>
								<img :src="ore.image" alt="" /><span v-if="showLayerNames">{{
									oreLabel(ore.kind)
								}}</span>
							</button>
						</ButtonStyled>
					</template>
					<span
						v-if="workspace.displayMode === 'ores' && dimensionOres.length === 0"
						class="layer-empty"
					>
						{{ formatMessage(messages.noOresInEnd) }}
					</span>
					<ButtonStyled
						v-if="workspace.dimension === 'overworld'"
						type="standard"
						:color="workspace.showSpawn ? 'brand' : 'standard'"
					>
						<button
							v-tooltip="formatMessage(messages.spawnPoint)"
							class="layer-option w-10 p-1"
							:class="{ named: showLayerNames }"
							:aria-pressed="workspace.showSpawn"
							@click="workspace.showSpawn = !workspace.showSpawn"
						>
							<img :src="`${structureAssetRoot}/spawn_point.webp`" alt="" /><span
								v-if="showLayerNames"
								>{{ formatMessage(messages.spawnPoint) }}</span
							>
						</button>
					</ButtonStyled>
				</div>
				<div v-if="layersExpanded" class="layer-expanded-panel">
					<div class="layer-bulk-actions">
						<ButtonStyled size="small" type="outlined"
							><button
								@click="
									workspace.displayMode === 'structures' ? selectAllFeatures() : selectAllOres()
								"
							>
								{{ formatMessage(messages.selectAll) }}
							</button></ButtonStyled
						>
						<ButtonStyled size="small" type="outlined"
							><button
								@click="workspace.displayMode === 'structures' ? clearFeatures() : clearOres()"
							>
								{{ formatMessage(messages.clear) }}
							</button></ButtonStyled
						>
						<ButtonStyled size="small" type="outlined"
							><button
								@click="workspace.displayMode === 'structures' ? resetFeatures() : resetOres()"
							>
								{{ formatMessage(messages.reset) }}
							</button></ButtonStyled
						>
					</div>
					<div class="layer-checklist">
						<template v-if="workspace.displayMode === 'structures'">
							<Checkbox
								v-for="feature in dimensionFeatures"
								:key="feature.kind"
								:model-value="workspace.visibleFeatures.includes(feature.kind)"
								:disabled="tileScale > feature.maxScale"
								:description="featureTooltip(feature.kind, feature.maxScale)"
								@update:model-value="toggleFeature(feature.kind)"
							>
								<span class="layer-checklist-label flex min-w-0 items-center gap-2">
									<img
										v-if="featureImageSource(feature.kind)"
										:src="featureImageSource(feature.kind)"
										alt=""
									/><component :is="featureIcons[feature.kind]" v-else />
									<span>{{ featureLabel(feature.kind) }}</span>
								</span>
							</Checkbox>
						</template>
						<template v-else>
							<Checkbox
								v-for="ore in dimensionOres"
								:key="ore.kind"
								:model-value="workspace.selectedOres.includes(ore.kind)"
								:description="oreLabel(ore.kind)"
								@update:model-value="toggleOre(ore.kind)"
							>
								<span class="layer-checklist-label flex min-w-0 items-center gap-2">
									<img :src="ore.image" alt="" />
									<span>{{ oreLabel(ore.kind) }}</span>
									<small>Y {{ ore.yMin }}~{{ ore.yMax }}</small>
								</span>
							</Checkbox>
						</template>
						<Checkbox
							v-if="workspace.dimension === 'overworld'"
							:model-value="workspace.showSpawn"
							:description="formatMessage(messages.spawnPoint)"
							@update:model-value="workspace.showSpawn = !workspace.showSpawn"
						>
							<span class="layer-checklist-label flex min-w-0 items-center gap-2">
								<img :src="`${structureAssetRoot}/spawn_point.webp`" alt="" />
								<span>{{ formatMessage(messages.spawnPoint) }}</span>
							</span>
						</Checkbox>
					</div>
					<div
						v-if="workspace.displayMode === 'ores' && dimensionOres.length > 0"
						class="ore-range-controls"
					>
						<div class="ore-range-heading">
							<strong>{{ formatMessage(messages.oreYRange) }}</strong>
							<span>Y {{ oreYMinimum }} - {{ oreYMaximum }}</span>
							<ButtonStyled size="small" type="transparent">
								<button @click="resetOreYRange">{{ formatMessage(messages.resetYRange) }}</button>
							</ButtonStyled>
						</div>
						<label>
							<span>{{ formatMessage(messages.oreMinY) }}</span>
							<Slider
								v-model="oreYMinimum"
								:min="activeOreYRange[0]"
								:max="oreYMaximum"
								:step="1"
							/>
						</label>
						<label>
							<span>{{ formatMessage(messages.oreMaxY) }}</span>
							<Slider
								v-model="oreYMaximum"
								:min="oreYMinimum"
								:max="activeOreYRange[1]"
								:step="1"
							/>
						</label>
					</div>
				</div>
			</div>

			<div
				class="seed-map-canvas-shell relative flex-1 overflow-hidden rounded-b-[var(--radius-md)] min-h-[32rem] border-x border-b border-surface-5 bg-surface-2"
				data-onboarding-id="seed-map-canvas"
			>
				<canvas
					ref="mapCanvas"
					class="block size-full touch-none"
					:class="{ 'cursor-crosshair': rulerEnabled }"
					@pointerdown="onPointerDown"
					@pointermove="onPointerMove"
					@pointerup="onPointerUp"
					@pointercancel="onPointerUp"
					@wheel="onWheel"
					@dblclick="onDoubleClick"
					@contextmenu="onContextMenu"
				/>

				<div class="map-coordinate-ruler map-coordinate-ruler-x">
					<span v-for="offset in [-2, -1, 0, 1, 2]" :key="offset">{{
						Math.round(workspace.center.x + (offset * viewport.width * scale) / 4)
					}}</span>
				</div>
				<div class="map-coordinate-ruler map-coordinate-ruler-z">
					<span v-for="offset in [-1, 0, 1]" :key="offset">{{
						Math.round(workspace.center.z + (offset * viewport.height * scale) / 3)
					}}</span>
				</div>

				<div class="map-control-stack map-control-stack-left left-3">
					<ButtonStyled circular type="standard"
						><button
							v-tooltip="formatMessage(messages.zoomIn)"
							:aria-label="formatMessage(messages.zoomIn)"
							:disabled="workspace.zoom <= SEED_MAP_MIN_ZOOM"
							@click="zoomBy(-0.5)"
						>
							<PlusIcon /></button
					></ButtonStyled>
					<ButtonStyled circular type="standard"
						><button
							v-tooltip="formatMessage(messages.zoomOut)"
							:aria-label="formatMessage(messages.zoomOut)"
							:disabled="workspace.zoom >= SEED_MAP_SCALES.length - 1"
							@click="zoomBy(0.5)"
						>
							<MinusIcon /></button
					></ButtonStyled>
					<ButtonStyled circular type="standard"
						><button
							v-tooltip="formatMessage(messages.centerSpawn)"
							:aria-label="formatMessage(messages.centerSpawn)"
							:disabled="!spawn"
							@click="centerSpawn"
						>
							<CompassIcon /></button
					></ButtonStyled>
				</div>

				<div class="map-control-stack map-control-stack-right right-3">
					<ButtonStyled circular type="standard"
						><button
							v-tooltip="
								formatMessage(isFullscreen ? messages.exitFullscreen : messages.fullscreen)
							"
							:aria-label="
								formatMessage(isFullscreen ? messages.exitFullscreen : messages.fullscreen)
							"
							@click="toggleFullscreen"
						>
							<ContractIcon v-if="isFullscreen" /><ExpandIcon v-else /></button
					></ButtonStyled>
					<ButtonStyled circular :type="rulerEnabled ? 'standard' : 'outlined'"
						><button
							v-tooltip="formatMessage(messages.ruler)"
							:aria-label="formatMessage(messages.ruler)"
							:aria-pressed="rulerEnabled"
							@click="rulerEnabled = !rulerEnabled"
						>
							<ScaleIcon /></button
					></ButtonStyled>
					<ButtonStyled circular :type="advancedOpen ? 'standard' : 'outlined'"
						><button
							v-tooltip="formatMessage(messages.mapSettings)"
							:aria-label="formatMessage(messages.mapSettings)"
							:aria-pressed="advancedOpen"
							@click="advancedOpen = !advancedOpen"
						>
							<SettingsIcon /></button
					></ButtonStyled>
				</div>

				<div v-if="oreZoomLimited" class="map-status map-ore-status">
					<PickaxeIcon class="size-3" />{{ formatMessage(messages.zoomToScanOres) }}
				</div>
				<div
					v-else-if="workspace.displayMode === 'ores' && workspace.dimension === 'end'"
					class="map-status map-ore-status"
				>
					<PickaxeIcon class="size-3" />{{ formatMessage(messages.noOresInEnd) }}
				</div>
				<div v-else-if="mapBusy" class="map-status map-ore-status">
					<RefreshCwIcon class="size-3 animate-spin" />{{
						oreScanning
							? formatMessage(messages.scanningOres, {
									processed: oreScannedChunks,
									total: oreTotalChunks,
								})
							: formatMessage(messages.loading)
					}}
				</div>
				<div v-if="rulerDistance !== null" class="map-status map-ruler-status">
					<ScaleIcon class="size-3" />{{
						formatMessage(messages.distance, { distance: rulerDistance })
					}}<ButtonStyled circular size="small" type="transparent"
						><button
							v-tooltip="formatMessage(messages.clearRuler)"
							:aria-label="formatMessage(messages.clearRuler)"
							@click="rulerPoints = []"
						>
							<XIcon /></button
					></ButtonStyled>
				</div>

				<div v-if="selection && selectionVisible" class="map-popup" :style="selectionStyle">
					<div class="flex items-start justify-between gap-3">
						<div class="min-w-0">
							<strong class="block truncate text-sm text-contrast">
								<i
									v-if="selection.biome !== undefined && !selectedMarker"
									class="popup-biome-dot"
									:style="{ backgroundColor: biomeColorOf(selection.biome) }"
								></i
								>{{
									selection.feature
										? featureResultLabel(selection.feature)
										: selection.ore
											? oreLabel(selection.ore.ore)
											: selection.spawn
												? formatMessage(messages.spawnPoint)
												: selectedMarker?.name ||
													(selection.biome !== undefined
														? biomeDisplayName(selection.biome)
														: formatMessage(messages.location))
								}}
							</strong>
							<span class="text-xs text-secondary"
								>X {{ selection.x
								}}<template v-if="selection.ore">, Y {{ selection.ore.y }}</template
								>, Z {{ selection.z
								}}<template v-if="selection.biome !== undefined && !selectedMarker">
									· {{ formatMessage(messages.biomeTag) }}</template
								></span
							>
						</div>
						<ButtonStyled circular size="small" type="transparent"
							><button :aria-label="formatMessage(messages.closePanel)" @click="selection = null">
								<XIcon /></button
						></ButtonStyled>
					</div>
					<div v-if="selection.ore" class="ore-hit-details">
						<span :class="{ verified: selection.ore.verified }">
							{{
								formatMessage(selection.ore.verified ? messages.oreVerified : messages.oreLikely)
							}}
						</span>
						<span>{{
							formatMessage(messages.orePrecision, { precision: selection.ore.precision })
						}}</span>
						<span>{{
							formatMessage(messages.oreCoverage, {
								min: selection.ore.yMin,
								max: selection.ore.yMax,
							})
						}}</span>
					</div>
					<div v-if="markerDraftOpen" class="mt-3 grid grid-cols-[minmax(0,1fr)_2.25rem] gap-2">
						<StyledInput v-model="markerName" :placeholder="formatMessage(messages.markerName)" />
						<input
							v-model="markerColor"
							type="color"
							class="marker-color-input"
							:aria-label="formatMessage(messages.markerColor)"
						/>
					</div>
					<div class="mt-3 flex flex-wrap gap-2">
						<ButtonStyled v-if="markerDraftOpen" size="small" color="brand"
							><button @click="addMarker">
								<PinIcon />{{ formatMessage(messages.addMarker) }}
							</button></ButtonStyled
						>
						<ButtonStyled size="small" type="outlined"
							><button @click="copyTeleport">
								<ClipboardCopyIcon />{{ formatMessage(messages.copyTeleport) }}
							</button></ButtonStyled
						>
						<ButtonStyled v-if="!markerDraftOpen && !selectedMarker" size="small" type="outlined"
							><button @click="markerDraftOpen = true">
								<PlusIcon />{{ formatMessage(messages.addMarker) }}
							</button></ButtonStyled
						>
						<Checkbox
							v-if="selection.feature"
							v-model="selectedFeatureCompleted"
							class="feature-completed-checkbox"
							:label="formatMessage(messages.completed)"
						/>
						<Checkbox
							v-if="selection.ore"
							v-model="selectedOreCompleted"
							class="feature-completed-checkbox"
							:label="formatMessage(messages.mined)"
						/>
						<ButtonStyled v-if="selectedMarker" size="small" type="outlined"
							><button @click="removeMarker(selectedMarker.id)">
								<TrashIcon />{{ formatMessage(messages.removeMarker) }}
							</button></ButtonStyled
						>
					</div>
				</div>

				<div v-if="advancedOpen" class="advanced-panel">
					<div class="flex items-center justify-between gap-3">
						<strong class="text-sm text-contrast">{{ formatMessage(messages.mapSettings) }}</strong
						><ButtonStyled circular size="small" type="transparent"
							><button
								:aria-label="formatMessage(messages.closePanel)"
								@click="advancedOpen = false"
							>
								<XIcon /></button
						></ButtonStyled>
					</div>
					<label
						><span>{{ formatMessage(messages.chunkCoordinates) }}</span
						><Toggle v-model="workspace.showChunkCoordinates" small
					/></label>
					<label
						><span>{{ formatMessage(messages.contours) }}</span
						><Toggle v-model="contoursEnabled" :disabled="workspace.dimension === 'nether'" small
					/></label>
					<div class="mt-3 border-t border-surface-5 pt-3">
						<div class="mb-2 flex items-center justify-between">
							<strong class="text-sm text-contrast">{{ formatMessage(messages.markers) }}</strong
							><span class="text-xs text-secondary">{{ workspace.markers.length }}</span>
						</div>
						<p v-if="!workspace.markers.length" class="m-0 text-xs text-secondary">
							{{ formatMessage(messages.noMarkers) }}
						</p>
						<div v-else class="marker-list flex flex-col gap-1">
							<div v-for="marker in workspace.markers" :key="marker.id">
								<button class="marker-jump" @click="jumpToMarker(marker)">
									<i :style="{ backgroundColor: marker.color }"></i><span>{{ marker.name }}</span
									><small>{{ marker.x }}, {{ marker.z }}</small>
								</button>
								<ButtonStyled circular size="small" type="transparent"
									><button
										:aria-label="formatMessage(messages.removeMarker)"
										@click="removeMarker(marker.id)"
									>
										<TrashIcon /></button
								></ButtonStyled>
							</div>
						</div>
					</div>
				</div>

				<div class="map-bottom-toolbar">
					<SeedMapBiomePicker
						class="map-bottom-biomes"
						:dimension="workspace.dimension"
						:enabled="workspace.highlightBiomeEnabled"
						:highlighted-biomes="workspace.highlightedBiomes"
						@update:dimension="workspace.dimension = $event"
						@update:enabled="workspace.highlightBiomeEnabled = $event"
						@update:highlighted-biomes="workspace.highlightedBiomes = $event"
					/>
					<label
						v-tooltip="
							terrainEnabled ? formatMessage(messages.elevationLockedByTerrain) : undefined
						"
						class="map-pill map-elevation"
						:class="{ locked: terrainEnabled }"
					>
						<span>{{ formatMessage(messages.elevation) }}</span>
						<StyledInput
							v-model="elevationInput"
							type="number"
							size="small"
							:min="-64"
							:max="320"
							:disabled="terrainEnabled"
							wrapper-class="elevation-input w-20"
						/>
					</label>
					<div class="map-bottom-actions max-sm:w-full max-sm:flex-wrap max-sm:justify-start max-sm:ml-0">
						<ButtonStyled
							:type="terrainEnabled ? 'highlight-colored-text' : 'outlined'"
							color="brand"
							><button
								class="map-action"
								:disabled="!terrainSupported"
								:aria-pressed="terrainEnabled"
								@click="terrainEnabled = !terrainEnabled"
							>
								<LayersIcon />{{ formatMessage(messages.terrain) }}
							</button></ButtonStyled
						>
						<ButtonStyled
							:type="workspace.showGrid ? 'highlight-colored-text' : 'outlined'"
							color="brand"
							><button
								class="map-action"
								:aria-pressed="workspace.showGrid"
								@click="workspace.showGrid = !workspace.showGrid"
							>
								<GridIcon />{{ formatMessage(messages.showGrid) }}
							</button></ButtonStyled
						>
						<ButtonStyled v-if="workspace.displayMode === 'structures'" type="outlined"
							><button
								class="map-action"
								:disabled="!displayedFeatures.length"
								@click="focusNearestFeature"
							>
								<SearchIcon />{{ formatMessage(messages.searchNearby) }}
							</button></ButtonStyled
						>
					</div>
				</div>
			</div>
		</section>

		<SeedMapWorldImportModal ref="worldImportModal" @import="onWorldImported" />
		<SeedMapCopyrightModal ref="copyrightModal" />
	</main>
</template>

<style scoped>
.map-scale-badge {
	flex: 0 0 auto;
	min-width: 7.5rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-2);
	padding: 0.35rem 0.55rem;
	color: var(--color-text-secondary);
	font-size: 0.7rem;
	font-variant-numeric: tabular-nums;
	font-weight: 700;
	text-align: center;
}

.seed-map-toolbar {
	position: relative;
	z-index: 20;
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	background: var(--surface-2);
	padding: 0.75rem;
}

.toolbar-primary {
	display: grid;
	grid-template-columns: minmax(11rem, 0.8fr) minmax(7rem, 0.45fr) minmax(18rem, 1.75fr);
	gap: 0.75rem;
}

/*
 * The secondary row is a strict three-column grid whose controls all share
 * a 2.5rem height, so the dimension switch, both coordinate fields, Go, and
 * Share stay width- and baseline-aligned.
 */
.toolbar-secondary {
	display: grid;
	grid-template-columns: auto minmax(0, 1fr) auto;
	align-items: end;
	gap: 0.75rem;
	border-top: 1px solid var(--surface-5);
	padding-top: 0.75rem;
}

.control-group {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: 0.35rem;
}

.control-label {
	color: var(--color-text-secondary);
	font-size: 0.7rem;
	font-weight: 700;
	line-height: 1;
}

:deep(.seed-map-dropdown .animated-dropdown) {
	width: 100%;
	max-width: none;
}

.seed-input-row {
	display: grid;
	grid-template-columns: minmax(0, 1fr) auto auto auto;
	align-items: center;
	gap: 0.5rem;
}

.seed-history-menu {
	display: flex;
	width: min(23rem, calc(100vw - 1.5rem));
	max-height: min(24rem, calc(100dvh - 4rem));
	min-height: 0;
	flex-direction: column;
	gap: 0;
	overflow: hidden;
}

.seed-history-heading {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 0.5rem;
	border-bottom: 1px solid var(--seed-history-border, var(--surface-5));
	padding: 0.1rem 0.2rem 0.6rem;
	color: var(--color-text-primary);
	font-size: 0.8rem;
}

.seed-history-empty {
	margin: 0;
	padding: 1.25rem 0.5rem;
	color: var(--color-secondary);
	font-size: 0.78rem;
	text-align: center;
}

.seed-history-list {
	display: flex;
	min-height: 0;
	flex-direction: column;
	gap: 0.35rem;
	overflow-y: auto;
	overscroll-behavior: contain;
	padding: 0.6rem 0.2rem 0.1rem 0;
	scrollbar-color: var(--seed-history-border, var(--surface-5)) transparent;
	scrollbar-width: thin;
}

.seed-history-row {
	display: flex;
	align-items: center;
	gap: 0.25rem;
	border: 1px solid var(--seed-history-border, var(--surface-5));
	border-radius: var(--radius-sm);
	background: var(--seed-history-row-bg, var(--surface-4));
	padding-right: 0.25rem;
	transition:
		background-color 0.15s ease,
		border-color 0.15s ease,
		box-shadow 0.15s ease;
}

.seed-history-row:hover {
	border-color: color-mix(
		in srgb,
		var(--color-brand) 45%,
		var(--seed-history-border, var(--surface-5))
	);
	background: var(--seed-history-row-hover-bg, var(--surface-3));
}

.seed-history-row:focus-within {
	border-color: var(--color-brand);
	box-shadow: 0 0 0 2px var(--color-brand-highlight);
}

.seed-history-load {
	display: flex;
	min-width: 0;
	flex: 1;
	flex-direction: column;
	gap: 0.15rem;
	border: 0;
	border-radius: var(--radius-sm);
	background: transparent;
	padding: 0.5rem 0.6rem;
	cursor: pointer;
	text-align: left;
}

.seed-history-load:focus-visible {
	outline: none;
}

.seed-history-seed {
	overflow: hidden;
	color: var(--color-contrast);
	font-size: 0.82rem;
	font-variant-numeric: tabular-nums;
	font-weight: 700;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.seed-history-meta {
	overflow: hidden;
	color: var(--color-secondary);
	font-size: 0.68rem;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.seed-history-progress {
	display: inline-flex;
	align-items: center;
	border: 1px solid var(--color-brand-highlight);
	border-radius: var(--radius-sm);
	background: var(--color-brand-highlight);
	margin-left: 0.3rem;
	padding: 0 0.3rem;
	color: var(--color-brand);
	font-weight: 700;
}

.seed-history-remove {
	--_text: var(--color-secondary);
	--_icon: var(--color-secondary);
	--_hover-bg: var(--color-red-highlight);
	--_hover-text: var(--color-red);
	--_hover-icon: var(--color-red);
}

:global(.v-popper__popper.seed-map-history-popout) {
	--seed-history-panel-bg: var(--surface-2);
	--seed-history-row-bg: var(--surface-4);
	--seed-history-row-hover-bg: var(--surface-3);
	--seed-history-border: var(--surface-5);
	--seed-history-shadow: 0 18px 48px color-mix(in srgb, var(--surface-5) 55%, transparent);
	--_popper-arrow-bg: var(--seed-history-panel-bg);
	--_popper-arrow-border: var(--seed-history-border);
	z-index: 10050 !important;
}

:global(html.dark-mode .v-popper__popper.seed-map-history-popout),
:global(html.oled-mode .v-popper__popper.seed-map-history-popout) {
	--seed-history-panel-bg: color-mix(in srgb, var(--surface-4) 88%, var(--color-text-primary));
	--seed-history-row-bg: color-mix(in srgb, var(--surface-5) 82%, var(--color-text-primary));
	--seed-history-row-hover-bg: color-mix(in srgb, var(--surface-5) 70%, var(--color-text-primary));
	--seed-history-border: color-mix(in srgb, var(--surface-5) 80%, var(--color-text-primary));
	--seed-history-shadow: 0 18px 48px color-mix(in srgb, var(--surface-5) 38%, transparent);
}

:global(.v-popper__popper.seed-map-history-popout .v-popper__inner) {
	border-color: var(--seed-history-border) !important;
	background-color: var(--seed-history-panel-bg) !important;
	box-shadow: var(--seed-history-shadow) !important;
}

.popup-biome-dot {
	display: inline-block;
	width: 0.6rem;
	height: 0.6rem;
	margin-right: 0.4rem;
	border: 1px solid var(--surface-5);
	border-radius: 50%;
}

.dimension-control :deep(.btn-wrapper > button.dimension-option) {
	height: 100%;
	min-width: 5.5rem;
	justify-content: center;
	border-radius: 0;
}

.dimension-control :deep(.btn-wrapper:first-child button) {
	border-radius: calc(var(--radius-md) - 1px) 0 0 calc(var(--radius-md) - 1px);
}

.dimension-control :deep(.btn-wrapper:last-child button) {
	border-radius: 0 calc(var(--radius-md) - 1px) calc(var(--radius-md) - 1px) 0;
}

.coordinate-control {
	display: grid;
	grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
	align-items: center;
	gap: 0.5rem;
}

.coordinate-control :deep(input) {
	height: 2.5rem;
}

.coordinate-field > span {
	position: absolute;
	z-index: 2;
	left: 0.65rem;
	color: var(--color-text-secondary);
	font-size: 0.7rem;
	font-weight: 800;
	pointer-events: none;
}

.coordinate-field :deep(.relative) {
	width: 100%;
}

.coordinate-field :deep(input) {
	padding-left: 1.75rem;
}

.share-button :deep(button) {
	height: 2.5rem;
}

.seed-map-experience {
	display: flex;
	flex-direction: column;
	border-radius: var(--radius-md);
	outline: none;
}

.seed-map-experience:fullscreen {
	height: 100vh;
	padding: 0.75rem;
}

.layer-strip {
	position: relative;
	z-index: 10;
	display: grid;
	grid-template-columns: auto minmax(0, 1fr);
	align-items: center;
	gap: 0.5rem;
	min-height: 4rem;
	border: 1px solid var(--surface-5);
	padding: 0.45rem 0.6rem;
	border-radius: var(--radius-md) var(--radius-md) 0 0;
	background: var(--surface-3);
	box-shadow: 0 8px 20px rgb(0 0 0 / 0.2);
}

/*
 * The expanded checklist floats above the map canvas so its options are never
 * covered by map overlays and the map does not shift while it is open.
 */
.layer-expanded-panel {
	position: absolute;
	z-index: 30;
	top: calc(100% + 0.4rem);
	right: -1px;
	left: -1px;
	display: flex;
	max-height: min(26rem, 56vh);
	flex-direction: column;
	gap: 0.5rem;
	overflow: hidden;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	background: var(--surface-3);
	box-shadow: 0 16px 34px rgb(0 0 0 / 0.36);
	padding: 0.6rem;
}

.layer-strip-heading {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 0.25rem;
	padding-left: 0.25rem;
	color: var(--color-text-secondary);
	font-size: 0.75rem;
	font-weight: 800;
	text-transform: uppercase;
}

.layer-strip-heading > span {
	white-space: nowrap;
}

.layer-mode-switch {
	display: flex;
	align-items: center;
	gap: 0.2rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-2);
	padding: 0.2rem;
	text-transform: none;
}

.layer-mode-switch :deep(button) {
	border-radius: var(--radius-sm);
	white-space: nowrap;
}

.layer-bulk-actions {
	display: flex;
	flex-wrap: wrap;
	gap: 0.25rem;
}

.layer-options {
	display: flex;
	min-width: 0;
	gap: 0.4rem;
	overflow-x: auto;
	padding: 0.1rem;
	scrollbar-width: none;
}

.layer-options::-webkit-scrollbar {
	display: none;
}

.layer-checklist {
	display: grid;
	min-height: 0;
	flex: 1;
	grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
	gap: 0.35rem 0.75rem;
	overflow-y: auto;
	overscroll-behavior: contain;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-2);
	padding: 0.65rem;
}

.layer-checklist > :deep(button) {
	min-width: 0;
	border-radius: var(--radius-sm);
	padding: 0.35rem;
}

.layer-checklist > :deep(button:hover) {
	background: var(--surface-4);
}

.layer-checklist-label img,
.layer-checklist-label svg {
	width: 1.35rem;
	height: 1.35rem;
	flex: 0 0 auto;
	object-fit: contain;
}

.layer-checklist-label span {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.layer-checklist-label small {
	margin-left: auto;
	color: var(--color-text-secondary);
	font-size: 0.68rem;
	font-variant-numeric: tabular-nums;
}

.layer-empty {
	display: inline-flex;
	align-items: center;
	padding: 0.35rem 0.5rem;
	color: var(--color-text-secondary);
	font-size: 0.75rem;
}

.ore-range-controls {
	display: grid;
	grid-template-columns: repeat(2, minmax(0, 1fr));
	gap: 0.65rem 1rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-2);
	padding: 0.65rem;
}

.ore-range-heading {
	display: flex;
	grid-column: 1 / -1;
	align-items: center;
	gap: 0.65rem;
	color: var(--color-text-primary);
	font-size: 0.75rem;
}

.ore-range-heading span {
	color: var(--color-text-secondary);
	font-variant-numeric: tabular-nums;
}

.ore-range-heading > :last-child {
	margin-left: auto;
}

.ore-range-controls > label {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: 0.4rem;
	color: var(--color-text-secondary);
	font-size: 0.7rem;
	font-weight: 700;
}

.layer-option svg,
.layer-option img {
	width: 1.5rem;
	height: 1.5rem;
	flex: 0 0 auto;
	object-fit: contain;
}

.layer-option.named {
	width: auto;
	min-width: 8rem;
	justify-content: flex-start;
	padding: 0 0.65rem;
}

.layer-option.named span {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.seed-map-canvas-shell canvas {
	image-rendering: pixelated;
}

.map-control-stack {
	position: absolute;
	z-index: 7;
	top: 2.5rem;
	display: flex;
	flex-direction: column;
	gap: 0.3rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	background: var(--surface-3);
	box-shadow: 0 8px 20px rgb(0 0 0 / 0.28);
	padding: 0.35rem;
}

.map-control-stack :deep(button) {
	border-radius: var(--radius-sm);
}

.map-coordinate-ruler {
	pointer-events: none;
	position: absolute;
	z-index: 2;
	display: flex;
	color: var(--color-text-primary);
	font-size: 0.65rem;
	font-weight: 700;
}

.map-coordinate-ruler-x {
	inset: 0 3.5rem auto 3.5rem;
	justify-content: space-between;
}

.map-coordinate-ruler-x span,
.map-coordinate-ruler-z span {
	border: 1px solid var(--surface-5);
	border-radius: 0 0 var(--radius-sm) var(--radius-sm);
	background: var(--surface-3);
	padding: 0.15rem 0.35rem;
	box-shadow: 0 3px 8px rgb(0 0 0 / 0.24);
}

.map-coordinate-ruler-z {
	inset: 3rem auto 3rem 0;
	align-items: flex-start;
	flex-direction: column;
	justify-content: space-between;
}

.map-coordinate-ruler-z span {
	border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}

.map-status {
	position: absolute;
	z-index: 5;
	display: flex;
	align-items: center;
	gap: 0.35rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	background: var(--surface-3);
	padding: 0.35rem 0.55rem;
	color: var(--color-text-secondary);
	font-size: 0.72rem;
	font-weight: 700;
}

.map-status > button {
	display: inline-flex;
	width: 1.5rem;
	height: 1.5rem;
	align-items: center;
	justify-content: center;
	border: 0;
	background: transparent;
	padding: 0;
	color: var(--color-text-secondary);
}

.map-status > button svg {
	width: 0.9rem;
	height: 0.9rem;
}

.map-ruler-status {
	bottom: 5.25rem;
	left: 0.75rem;
}

.map-ore-status {
	top: 0.75rem;
	left: 50%;
	transform: translateX(-50%);
}

.map-popup,
.advanced-panel {
	position: absolute;
	z-index: 8;
	width: min(17rem, calc(100% - 1rem));
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	background: var(--surface-3);
	box-shadow: 0 12px 28px rgb(0 0 0 / 0.28);
	padding: 0.75rem;
}

.advanced-panel {
	right: 0.75rem;
	bottom: 4.3rem;
	max-height: min(28rem, calc(100% - 7rem));
	overflow-y: auto;
}

.advanced-panel > label {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 1rem;
	margin-top: 0.75rem;
	color: var(--color-text-primary);
	font-size: 0.8rem;
}

.marker-list > div {
	display: flex;
	align-items: center;
	gap: 0.35rem;
}

.marker-jump {
	display: grid;
	min-width: 0;
	flex: 1;
	grid-template-columns: auto minmax(0, 1fr) auto;
	align-items: center;
	gap: 0.45rem;
	border: 0;
	border-radius: var(--radius-sm);
	background: transparent;
	padding: 0.35rem;
	color: var(--color-text-primary);
	text-align: left;
}

.marker-jump:hover {
	background: var(--surface-4);
}

.marker-jump i {
	width: 0.6rem;
	height: 0.6rem;
	border-radius: 50%;
}

.marker-jump span {
	overflow: hidden;
	font-size: 0.75rem;
	font-style: normal;
	font-weight: 700;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.marker-jump small {
	color: var(--color-text-secondary);
	font-size: 0.65rem;
}

.feature-completed-checkbox {
	min-height: 2rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-4);
	padding: 0.35rem 0.55rem;
}

.ore-hit-details {
	display: flex;
	flex-wrap: wrap;
	gap: 0.35rem;
	margin-top: 0.6rem;
}

.ore-hit-details span {
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-4);
	padding: 0.2rem 0.4rem;
	color: var(--color-text-secondary);
	font-size: 0.68rem;
	font-weight: 700;
}

.ore-hit-details span.verified {
	border-color: var(--color-brand-highlight);
	background: var(--color-brand-highlight);
	color: var(--color-brand);
}

/*
 * One flat row of same-height controls: the biome cluster grows, the height
 * pill stays compact, and the action buttons on the right share one width so
 * the bar reads as a single aligned toolbar instead of mismatched boxes.
 */
.map-bottom-toolbar {
	position: absolute;
	z-index: 6;
	right: 0.75rem;
	bottom: 0.75rem;
	left: 0.75rem;
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.5rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	background: var(--surface-3);
	padding: 0.5rem 0.6rem;
	box-shadow: 0 10px 26px rgb(0 0 0 / 0.3);
}

.map-bottom-biomes {
	min-width: min(20rem, 100%);
	flex: 1 1 20rem;
}

.map-pill {
	display: flex;
	height: 2.5rem;
	flex: 0 0 auto;
	align-items: center;
	gap: 0.5rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-md);
	background: var(--surface-4);
	padding: 0 0.6rem;
	color: var(--color-text-primary);
	font-size: 0.75rem;
	font-weight: 700;
	white-space: nowrap;
}

.map-elevation.locked {
	color: var(--color-text-secondary);
}

.map-bottom-actions {
	display: flex;
	flex: 0 0 auto;
	align-items: center;
	gap: 0.5rem;
	margin-left: auto;
}

.map-bottom-actions :deep(button.map-action) {
	min-width: 7.25rem;
	height: 2.5rem;
	justify-content: center;
	white-space: nowrap;
}

.marker-color-input {
	width: 2.25rem;
	height: 2.25rem;
	border: 1px solid var(--surface-5);
	border-radius: var(--radius-sm);
	background: var(--surface-4);
	padding: 0.2rem;
}

@media (max-width: 1200px) {
	.toolbar-primary {
		grid-template-columns: minmax(11rem, 1fr) minmax(8rem, 0.7fr);
	}

	.seed-control {
		grid-column: 1 / -1;
	}

	.toolbar-secondary {
		grid-template-columns: auto minmax(0, 1fr);
	}

	.share-button {
		grid-column: 1 / -1;
		justify-self: end;
	}

	.coordinate-group {
		min-width: min(100%, 22rem);
	}

	.map-ruler-status {
		bottom: 8.5rem;
	}

	.advanced-panel {
		bottom: 7.5rem;
	}
}

@media (max-width: 900px) {
	.seed-map-page {
		height: auto;
		min-height: 100%;
	}

	.toolbar-primary {
		display: flex;
		flex-direction: column;
	}

	.toolbar-secondary {
		grid-template-columns: minmax(0, 1fr);
	}

	.dimension-group,
	.coordinate-group {
		width: 100%;
		min-width: 0;
	}

	.dimension-control,
	.coordinate-control {
		max-width: 100%;
		flex-wrap: wrap;
	}

	.coordinate-field {
		min-width: 7rem;
	}

	.share-button {
		margin-left: 0;
	}

	.share-button :deep(button) {
		width: 100%;
	}

	.layer-strip {
		grid-template-columns: 1fr;
	}

	.layer-strip-heading {
		flex-wrap: wrap;
		justify-content: space-between;
	}

	.layer-mode-switch {
		order: 3;
		width: 100%;
	}

	.layer-mode-switch :deep(.button-base),
	.layer-mode-switch :deep(button) {
		flex: 1;
	}

	.seed-map-canvas-shell {
		min-height: 32rem;
	}
}

@media (max-width: 640px) {
	.layer-checklist {
		grid-template-columns: minmax(0, 1fr);
	}

	.map-bottom-toolbar {
		max-height: calc(100% - 5rem);
		overflow-y: auto;
	}

	.map-ruler-status {
		bottom: 12rem;
	}

	.advanced-panel {
		bottom: 10.5rem;
	}
	.ore-range-controls {
		grid-template-columns: minmax(0, 1fr);
	}
}
</style>
