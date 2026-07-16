import type { CurseForgeCategory } from '@/helpers/curseforge'

export const CF_EXTRA_CATEGORY_HEADER = 'cf-extra'
export const CF_CATEGORY_VALUE_PREFIX = 'cf:'

/**
 * Hand-maintained Modrinth category slug → CurseForge category slug aliases.
 * Runtime resolution still prefers exact slug matches against live CF categories.
 */
const MODRINTH_TO_CURSEFORGE_SLUGS: Record<string, string[]> = {
	// Keys are Modrinth category slugs. Values are live CurseForge slugs.
	// Prefer one primary CF category so multi-select does not AND-filter to empty.
	adventure: ['adventure-rpg', 'adventure-and-rpg', 'adventure'],
	atmosphere: ['fantasy', 'realistic'],
	audio: ['sound'],
	blocks: ['world-gen'],
	bloom: ['realistic', 'fantasy'],
	cartoon: ['traditional', 'fantasy'],
	challenging: ['hardcore', 'expert'],
	combat: ['armor-weapons-tools', 'combat-pvp'],
	'core-shaders': ['realistic', 'fantasy'],
	cursed: ['mc-miscellaneous', 'miscellaneous'],
	decoration: ['cosmetic'],
	economy: ['economy'],
	entities: ['world-mobs', 'mobs'],
	environment: ['world-gen', 'world-biomes'],
	equipment: ['armor-weapons-tools'],
	fantasy: ['fantasy', 'magic', 'adventure-and-rpg'],
	foliage: ['world-biomes', 'world-gen'],
	fonts: ['font-packs'],
	food: ['mc-food', 'food'],
	'game-mechanics': ['mc-miscellaneous', 'mechanics'],
	gui: ['map-information'],
	items: ['armor-weapons-tools'],
	'kitchen-sink': ['extra-large', 'multiplayer'],
	library: ['library-api', 'library'],
	lightweight: ['small-light', 'performance', 'vanilla'],
	locale: ['font-packs'],
	magic: ['magic'],
	management: ['server-utility', 'admin-tools'],
	minigame: ['mini-game', 'fun'],
	mobs: ['world-mobs', 'mobs'],
	modded: ['mod-support'],
	models: ['cosmetic'],
	multiplayer: ['multiplayer'],
	optimization: ['performance'],
	'path-tracing': ['realistic', 'photo-realistic'],
	pve: ['adventure-and-rpg', 'adventure-rpg', 'hardcore'],
	pvp: ['combat-pvp', 'armor-weapons-tools'],
	quests: ['quests', 'adventure-and-rpg'],
	realistic: ['realistic', 'photo-realistic'],
	simplistic: ['traditional', 'vanilla', 'small-light'],
	social: ['multiplayer', 'chat-related'],
	storage: ['storage'],
	technology: ['technology', 'tech'],
	themed: ['steampunk', 'medieval', 'modern'],
	transportation: ['technology-player-transport', 'tech', 'technology'],
	tweaks: ['utility-qol', 'vanilla'],
	utility: ['utility-qol', 'utility'],
	'vanilla-like': ['vanilla', 'traditional', 'small-light'],
	worldgen: ['world-gen', 'exploration'],
}

/**
 * CurseForge category slug / English name → Simplified Chinese.
 * Keys are normalized (lowercase, hyphenated). Covers common Minecraft CF classes.
 */
const CF_NAME_TRANSLATIONS: Record<string, string> = {
	addon: '附加内容',
	addons: '附加内容',
	advanced: '进阶',
	adventure: '冒险',
	ae2: '应用能源 2',
	aether: '天境',
	age: '时代',
	ages: '时代',
	agriculture: '农业',
	anarchy: '无政府',
	animated: '动态',
	api: 'API',
	atm: 'ATM 系列',
	atmosphere: '氛围',
	atmospheric: '氛围',
	audio: '音频',
	automation: '自动化',
	beginner: '新手友好',
	beginners: '新手友好',
	biomes: '生物群系',
	blocks: '方块',
	bloom: '泛光',
	botania: '植物魔法',
	builders: '建筑',
	building: '建筑',
	bukkit: 'Bukkit',
	bungeecord: 'BungeeCord',
	campaign: '战役',
	cartoon: '卡通',
	categories: '分类',
	category: '分类',
	cave: '洞穴',
	caves: '洞穴',
	challenge: '挑战',
	challenging: '高挑战',
	chat: '聊天',
	cobblemon: '宝可梦',
	colored: '彩色',
	combat: '战斗',
	community: '社区',
	coop: '合作',
	cosmetic: '装饰',
	cosmetics: '装饰',
	crafttweaker: 'CraftTweaker',
	create: '机械动力',
	creative: '创造',
	ctm: '连接纹理',
	cursed: '诅咒',
	customization: '自定义',
	datapack: '数据包',
	datapacks: '数据包',
	decoration: '装饰',
	difficult: '困难',
	difficulty: '难度',
	dimension: '维度',
	dimensions: '维度',
	easy: '简单',
	economy: '经济',
	education: '教育',
	emi: 'EMI',
	end: '末地',
	energy: '能源',
	enigmatica: 'Enigmatica',
	entities: '实体',
	equipment: '装备',
	expert: '专家难度',
	exploration: '探索',
	fabric: 'Fabric',
	fantasy: '奇幻',
	farming: '农业',
	fixed: '固定',
	fonts: '字体',
	food: '食物',
	forestry: '林业',
	forge: 'Forge',
	ftb: 'FTB',
	fun: '娱乐',
	galacticraft: '星系',
	gamestages: '游戏阶段',
	general: '通用',
	genetics: '基因',
	gregtech: '格雷科技',
	gtnh: 'GTNH',
	gui: '界面',
	hard: '困难',
	hardcore: '极限',
	heavy: '重度',
	high: '高配',
	horror: '恐怖',
	hqm: '极限任务',
	hybrid: '混合',
	ic2: '工业 2',
	immersive: '沉浸工程',
	industrial: '工业',
	information: '信息',
	informational: '信息',
	integration: '集成',
	intermediate: '中等',
	items: '物品',
	jei: 'JEI',
	kubejs: 'KubeJS',
	lan: '局域网',
	languages: '语言',
	large: '大型',
	library: '支持库',
	light: '轻量',
	lightweight: '轻量',
	liteloader: 'LiteLoader',
	locales: '语言',
	localization: '本地化',
	magic: '魔法',
	mechanics: '机制',
	medieval: '中世纪',
	medium: '中型',
	mega: '大型',
	mekanism: '通用机械',
	minigame: '小游戏',
	minigames: '小游戏',
	misc: '杂项',
	miscellaneous: '杂项',
	mixed: '综合',
	mobs: '生物',
	mod: '模组',
	modded: '模组化',
	models: '模型',
	modern: '现代',
	modpack: '整合包',
	modpacks: '整合包',
	mods: '模组',
	multiplayer: '多人',
	neoforge: 'NeoForge',
	nether: '下界',
	nomifactory: '诺米工厂',
	oceanblock: '海岛',
	official: '官方',
	op: '超模',
	optimization: '优化',
	options: '选项',
	ores: '矿石',
	overpowered: '超模',
	paper: 'Paper',
	pbr: 'PBR',
	performance: '性能优化',
	photorealistic: '写实',
	pixelmon: '神奇宝贝',
	plugin: '插件',
	plugins: '插件',
	plugman: '插件管理',
	potato: '低配',
	processing: '加工',
	progression: '进度导向',
	prominence: 'Prominence',
	protection: '保护',
	purpur: 'Purpur',
	pve: 'PvE',
	pvp: 'PvP',
	qol: '生活质量',
	quest: '任务',
	quests: '任务',
	quilt: 'Quilt',
	realistic: '写实',
	redstone: '红石',
	rei: 'REI',
	resourcepacks: '资源包',
	rift: 'Rift',
	rlcraft: 'RLCraft',
	roleplay: '角色扮演',
	rpg: 'RPG',
	science: '科学',
	scifi: '科幻',
	server: '服务器',
	shader: '光影',
	shaders: '光影',
	simplistic: '简约',
	singleplayer: '单人',
	skyblock: '空岛',
	skyfactory: '天空工厂',
	small: '小型',
	smp: '多人生存',
	sound: '音效',
	space: '太空',
	spigot: 'Spigot',
	steampunk: '蒸汽朋克',
	stoneblock: '石块空岛',
	storage: '存储',
	story: '剧情',
	structures: '结构',
	survival: '生存',
	tech: '科技',
	technology: '科技',
	teleportation: '传送',
	thaumcraft: '神秘时代',
	themed: '主题',
	thermal: '热力系列',
	tinkers: '匠魂',
	traditional: '传统',
	transport: '运输',
	transportation: '交通',
	tweaks: '微调',
	twilight: '暮色',
	twitch: 'Twitch',
	ultra: '极致',
	utility: '实用',
	valhelsia: 'Valhelsia',
	vanilla: '原版+',
	velocity: 'Velocity',
	waterfall: 'Waterfall',
	world: '世界',
	worldgen: '世界生成',
	worlds: '世界',
	'128x': '128x',
	'16x': '16x',
	'256x': '256x',
	'32x': '32x',
	'512x': '512x',
	'512x-and-higher': '512x+',
	'512x-plus': '512x+',
	'64x': '64x',
	'admin-tools': '管理工具',
	'adventure-and-rpg': '冒险与 RPG',
	'adventure-maps': '冒险地图',
	'adventure-rpg': '冒险与 RPG',
	'adventure-worlds': '冒险世界',
	'all-the-mods': 'ATM 系列',
	'anti-griefing': '反破坏',
	'anti-griefing-tools': '反破坏',
	'api-and-library': '支持库与 API',
	'applied-energistics-2': '应用能源 2',
	'armor-tools-and-weapons': '盔甲、工具与武器',
	'armor-tools-weapons': '盔甲、工具与武器',
	'armor-weapons-tools': '盔甲、工具与武器',
	'better-minecraft': 'Better Minecraft',
	'blood-magic': '血魔法',
	'bug-fix': '漏洞修复',
	'bug-fixes': '漏洞修复',
	'co-op': '合作',
	'colored-lighting': '彩色光照',
	'combat-pvp': '战斗 / PvP',
	'connected-textures': '连接纹理',
	'core-shaders': '核心光影',
	'create-based': '机械动力向',
	'creation-worlds': '创造世界',
	'data-packs': '数据包',
	'data-packs-and-scripts': '数据包与脚本',
	'developer-tools': '开发工具',
	'divine-journey': '神圣之旅',
	'economy-plugins': '经济插件',
	'exploration-adventure': '探索冒险',
	'extra-large': '超大型',
	'fixed-inventory': '固定物品栏',
	'font-packs': '字体包',
	'food-and-farming': '食物与农业',
	'food-farming': '食物与农业',
	'ftb-official-pack': 'FTB 官方整合包',
	'ftb-quests': 'FTB 任务',
	'game-map': '游戏地图',
	'game-mechanics': '游戏机制',
	'genetic-engineering': '基因工程',
	'hardcore-questing': '极限任务',
	'immersive-engineering': '沉浸工程',
	'industrial-craft': '工业',
	'kitchen-sink': '大杂烩',
	'kitchen-sinks': '大杂烩',
	'library-api': '支持库与 API',
	'magic-based': '魔法向',
	'map-and-information': '地图与信息',
	'map-based': '基于地图',
	'map-information': '地图与信息',
	'mc-frp': 'MC FRP',
	'mini-game': '小游戏',
	'mod-support': '模组支持',
	'modded-worlds': '模组世界',
	'ores-resources': '矿石与资源',
	'parkour-maps': '跑酷地图',
	'path-tracing': '路径追踪',
	'photo-realistic': '写实',
	'player-transport': '玩家运输',
	'puzzle-maps': '解谜地图',
	'quality-of-life': '生活质量',
	'ray-tracing': '光线追踪',
	'resource-packs': '资源包',
	'role-playing': '角色扮演',
	'sci-fi': '科幻',
	'server-pack': '服务器包',
	'server-ready': '服务器就绪',
	'server-utility': '服务器实用',
	'sevs-tech': 'SevTech',
	'single-player': '单人',
	'sky-block': '空岛',
	'sky-factory': '天空工厂',
	'small-light': '小型 / 轻量',
	'story-driven': '剧情向',
	'survival-maps': '生存地图',
	'tech-and-magic': '科技与魔法',
	'tech-based': '科技向',
	'tech-magic': '科技魔法',
	'thermal-expansion': '热力膨胀',
	'tinkers-construct': '匠魂',
	'twilight-forest': '暮色森林',
	'twitch-integration': 'Twitch 集成',
	'utility-and-qol': '实用与生活质量',
	'utility-qol': '实用与生活质量',
	'vanilla-like': '原版风格',
	'vanilla-plus': '原版+',
	'vault-hunters': 'Vault Hunters',
	'website-administration': '网站管理',
	'world-editing': '世界编辑',
	'world-editing-and-management': '世界编辑与管理',
	'world-gen': '世界生成',
	'world-generation': '世界生成',
	'world-generators': '世界生成器',
	'world-management': '世界管理',
}

function normalizeSlug(value: string): string {
	return value
		.trim()
		.toLowerCase()
		.replace(/&/g, 'and')
		.replace(/['"]/g, '')
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
}

function uniqueNumbers(values: number[]): number[] {
	return [...new Set(values.filter((value) => Number.isFinite(value)))]
}

function lookupTranslation(...rawValues: Array<string | null | undefined>): string | undefined {
	for (const raw of rawValues) {
		if (!raw) continue
		const key = normalizeSlug(raw)
		if (!key) continue
		const hit = CF_NAME_TRANSLATIONS[key]
		if (hit) return hit
	}
	return undefined
}

export function isCurseForgeOnlyCategoryName(name: string): boolean {
	return name.startsWith(CF_CATEGORY_VALUE_PREFIX)
}

export function parseCurseForgeCategoryValue(name: string): number | undefined {
	if (!isCurseForgeOnlyCategoryName(name)) return undefined
	const id = Number(name.slice(CF_CATEGORY_VALUE_PREFIX.length))
	return Number.isFinite(id) ? id : undefined
}

export function curseForgeCategoryValue(id: number): string {
	return `${CF_CATEGORY_VALUE_PREFIX}${id}`
}

export function localizeCurseForgeLabel(...rawValues: Array<string | null | undefined>): string {
	const translated = lookupTranslation(...rawValues)
	if (translated) return translated

	const fallback = rawValues.find((value) => Boolean(value && String(value).trim()))
	if (!fallback) return ''

	// Title-case leftover English labels so untranslated CF tags still look readable.
	return String(fallback)
		.replace(/[-_]+/g, ' ')
		.replace(/\s+/g, ' ')
		.trim()
		.replace(/\b\w/g, (char) => char.toUpperCase())
}

export function localizeCurseForgeCategoryName(
	category: Pick<CurseForgeCategory, 'name' | 'slug'>,
): string {
	return localizeCurseForgeLabel(category.slug, category.name)
}

export function localizeCurseForgeCategoryLabels(labels: string[] | undefined | null): string[] {
	if (!labels?.length) return []
	return labels.map((label) => localizeCurseForgeLabel(label))
}

export function buildCurseForgeCategoryIndex(categories: CurseForgeCategory[]) {
	const bySlug = new Map<string, CurseForgeCategory[]>()
	const byId = new Map<number, CurseForgeCategory>()

	for (const category of categories) {
		if (category.isClass) continue
		byId.set(category.id, category)
		const keys = [category.slug, category.name]
			.filter(Boolean)
			.map((value) => normalizeSlug(String(value)))
		for (const key of keys) {
			const list = bySlug.get(key) ?? []
			list.push(category)
			bySlug.set(key, list)
		}
	}

	return { bySlug, byId }
}

export function mapModrinthCategoryToCurseForgeIds(
	modrinthSlug: string,
	categories: CurseForgeCategory[],
): number[] {
	const { bySlug } = buildCurseForgeCategoryIndex(categories)
	const normalized = normalizeSlug(modrinthSlug)

	// Exact CF slug/name first.
	const exact = bySlug.get(normalized) ?? []
	if (exact.length > 0) {
		return uniqueNumbers(exact.map((category) => category.id)).slice(0, 1)
	}

	// Otherwise use the first alias that exists in the live CF category list.
	// Returning multiple IDs can AND-filter CF search and yield empty pages.
	for (const alias of (MODRINTH_TO_CURSEFORGE_SLUGS[normalized] ?? []).map(normalizeSlug)) {
		const matches = bySlug.get(alias) ?? []
		if (matches.length > 0) {
			return uniqueNumbers(matches.map((category) => category.id)).slice(0, 1)
		}
	}

	return []
}

export function findUnmappedCurseForgeCategories(
	modrinthSlugs: string[],
	categories: CurseForgeCategory[],
): CurseForgeCategory[] {
	const mappedIds = new Set<number>()
	for (const slug of modrinthSlugs) {
		for (const id of mapModrinthCategoryToCurseForgeIds(slug, categories)) {
			mappedIds.add(id)
		}
	}

	// Also treat exact slug overlaps as mapped even without explicit table entries.
	const mrSlugSet = new Set(modrinthSlugs.map(normalizeSlug))
	return categories.filter((category) => {
		if (category.isClass) return false
		if (mappedIds.has(category.id)) return false
		const slug = normalizeSlug(category.slug || category.name)
		return !mrSlugSet.has(slug)
	})
}

export function resolveCurseForgeCategoryIdsFromFilterValues(
	values: string[],
	categories: CurseForgeCategory[],
	loaderSlugs: Set<string>,
): number[] {
	const { bySlug, byId } = buildCurseForgeCategoryIndex(categories)
	const ids: number[] = []

	for (const value of values) {
		const normalized = normalizeSlug(value)
		if (!normalized || loaderSlugs.has(normalized) || loaderSlugs.has(value)) continue

		const prefixedId = parseCurseForgeCategoryValue(value)
		if (prefixedId !== undefined) {
			if (byId.has(prefixedId) || categories.some((category) => category.id === prefixedId)) {
				ids.push(prefixedId)
			}
			continue
		}

		// Numeric category ids (rare, but keep as a fallback).
		if (/^\d+$/.test(value)) {
			const numericId = Number(value)
			if (byId.has(numericId) || categories.some((category) => category.id === numericId)) {
				ids.push(numericId)
				continue
			}
		}

		// Prefer exact CF slug/name matches from the live category list.
		const directMatches = bySlug.get(normalized) ?? []
		if (directMatches.length > 0) {
			ids.push(...directMatches.map((category) => category.id))
			continue
		}

		// Fall back to Modrinth slug → CF alias mapping for unified "all sources" mode.
		ids.push(...mapModrinthCategoryToCurseForgeIds(value, categories))
	}

	return uniqueNumbers(ids).slice(0, 10)
}
