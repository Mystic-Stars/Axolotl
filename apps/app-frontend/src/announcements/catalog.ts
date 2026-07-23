export type AnnouncementLocale = 'en-US' | 'zh-CN'

export type AnnouncementChangeType =
	| 'added'
	| 'changed'
	| 'deprecated'
	| 'removed'
	| 'fixed'
	| 'security'

export type LocalizedAnnouncementText = Readonly<Record<AnnouncementLocale, string>>

export type AnnouncementChange = LocalizedAnnouncementText

export type LauncherAnnouncement = {
	readonly id: string
	readonly version: string
	readonly publishedAt: string
	readonly title: LocalizedAnnouncementText
	readonly changes: Readonly<Partial<Record<AnnouncementChangeType, readonly AnnouncementChange[]>>>
	readonly notes?: LocalizedAnnouncementText
	readonly externalUrl?: string
}

export const ANNOUNCEMENT_CHANGE_TYPES: readonly AnnouncementChangeType[] = [
	'added',
	'changed',
	'deprecated',
	'removed',
	'fixed',
	'security',
]

export const launcherAnnouncements: readonly LauncherAnnouncement[] = [
	{
		id: 'launcher-1.5.1',
		version: '1.5.1',
		publishedAt: '2026-07-23',
		title: {
			'en-US': 'Axolotl Launcher 1.5.1',
			'zh-CN': 'Axolotl Launcher 1.5.1',
		},
		changes: {
			added: [
				{
					'en-US':
						'Expanded Java detection to search JAVA_HOME sibling installations, common vendor locations, official Minecraft Launcher runtimes, and likely installation folders.',
					'zh-CN':
						'扩展 Java 自动检测范围，现可搜索 JAVA_HOME 同级安装、常见发行版目录、Minecraft 官方启动器运行时及可能的安装目录。',
				},
			],
			changed: [
				{
					'en-US':
						'Java detection now caches results, scans sources concurrently, and refreshes the installation list in the background.',
					'zh-CN':
						'Java 检测现在会缓存结果、并行扫描不同来源，并在后台刷新安装列表。',
				},
				{
					'en-US':
						'The launcher now reuses an already detected Java runtime with the required version before downloading a new one.',
					'zh-CN':
						'启动实例缺少所需 Java 版本时，现在会优先复用已检测到的同版本运行时，再考虑下载新的运行时。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed Java detection for several Windows registry paths and nested Eclipse Adoptium installation entries.',
					'zh-CN':
						'修复部分 Windows 注册表路径及 Eclipse Adoptium 嵌套安装项无法检测 Java 的问题。',
				},
			],
		},
	},

	{
		id: 'launcher-1.5.0',
		version: '1.5.0',
		publishedAt: '2026-07-23',
		title: {
			'en-US': 'Axolotl Launcher 1.5.0',
			'zh-CN': 'Axolotl Launcher 1.5.0',
		},
		changes: {
			added: [
				{
					'en-US':
						'Added HMCL, PCL2, and PCL2CE launcher instance import — all instances are now discovered and imported directly from these launchers.',
					'zh-CN':
						'新增 HMCL、PCL2、PCL2CE 启动器实例导入支持，可直接根据启动器解析出所有实例。',
				},
				{
					'en-US':
						'Added generic folder import — any directory containing a .minecraft folder can now be imported as an instance.',
					'zh-CN':
						'新增通用文件夹导入功能，可导入任意含 .minecraft 的目录。',
				},
				{
					'en-US':
						'Added "import as shared instance" support, optionally using symlinks instead of copying to save disk space.',
					'zh-CN':
						'新增添加为共享实例功能：导入时可选软链接而非复制。',
				},
				{
					'en-US':
						'Added a confirmation dialog when deleting files from the file browser tab.',
					'zh-CN':
						'补齐文件标签页删除时的确认弹窗。',
				},
				{
					'en-US':
						'Added OptiFine support — declared OptiFine in a modpack is automatically installed; standalone, or as a mod alongside other loaders.',
					'zh-CN':
						'新增 OptiFine 支持：整合包声明 OptiFine 时自动安装——单独存在时作为加载器。',
				},
			],
			changed: [
				{
					'en-US':
						'Optimised copy_dotminecraft_with_reporter: serial copies are now concurrent, reducing time complexity from O(n·t) to O(max(t)), and progress reporting has been improved.',
					'zh-CN':
						'优化 copy_dotminecraft_with_reporter：串行复制改为并发，时间复杂度由 O(n·t) 降为 O(max(t))，优化进度上报时机。',
				},
				{
					'en-US':
						'Updated shared instance indicators and warning hints for clarity.',
					'zh-CN':
						'更新共享实例标识与警告提示。',
				},
				{
					'en-US':
						'Greatly improved modpack import compatibility — now handles CurseForge, MCBBS, HMCL, MultiMC, PCL launcher-bundled archives and various non-standard pack formats.',
					'zh-CN':
						'大大增强整合包导入兼容性，兼容 CurseForge、MCBBS、HMCL、MultiMC、PCL 等导出的附带启动器的整合包以及各种不完全符合规范的整合包格式。',
				},
			],
		},
	},

	{
		id: 'launcher-1.4.1',
		version: '1.4.1',
		publishedAt: '2026-07-23',
		title: {
			'en-US': 'Axolotl Launcher 1.4.1',
			'zh-CN': 'Axolotl Launcher 1.4.1',
		},
		changes: {
			added: [
				{
					'en-US':
						'Modpack imports now detect the archive format by content: CurseForge, MCBBS, HMCL, and MultiMC/Prism export packs, launcher-bundled archives, and zipped game folders can be imported alongside .mrpack files.',
					'zh-CN':
						'整合包导入现在按压缩包内容识别格式：除 .mrpack 外，还支持 CurseForge、MCBBS、HMCL、MultiMC/Prism 导出包、附带启动器的整合包以及打包的游戏目录。',
				},
				{
					'en-US':
						'Added OptiFine support: modpacks declaring OptiFine install it automatically, standalone as the loader or as a mod alongside Forge/NeoForge.',
					'zh-CN':
						'新增 OptiFine 支持：声明了 OptiFine 的整合包会自动安装——单独存在时作为加载器，与 Forge/NeoForge 共存时作为模组安装。',
				},
				{
					'en-US':
						'Added an appearance setting to limit the number of recent instances shown in the sidebar, with 0 showing all instances.',
					'zh-CN': '新增外观设置，可限制侧边栏显示的最近实例数量，设为 0 时显示全部实例。',
				},
				{
					'en-US':
						'Added custom accent colors with a preset palette, hue slider, hex input, and automatic light and dark theme variants.',
					'zh-CN':
						'新增自定义强调色，支持预设色板、色相滑块、十六进制色号及自动生成浅色和深色主题变体。',
				},
			],
			changed: [
				{
					'en-US': 'Improved the update settings version history with clearer release cards and details.',
					'zh-CN': '优化更新设置中的版本历史，提供更清晰的发布卡片和详情展示。',
				},
				{
					'en-US':
						'The sidebar instance list now scrolls independently when it exceeds the available space.',
					'zh-CN': '侧边栏实例列表超出可用空间时，现在可以独立滚动。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed the quick instance switcher failing to render when the instance list could not be loaded.',
					'zh-CN': '修复实例列表加载失败时快速实例切换器无法显示的问题。',
				},
				{
					'en-US':
						'Fixed local modpack installs appearing stuck at 100% and hanging when a Minecraft file download stops receiving data.',
					'zh-CN':
						'修复本地整合包安装在 100% 后看似卡住，以及 Minecraft 文件下载停止接收数据时任务无法结束的问题。',
				},
				{
					'en-US':
						'Fixed the Minecraft download progress overshooting and pegging at 100% early after a download attempt was retried.',
					'zh-CN': '修复下载重试后 Minecraft 资源下载进度虚高、提前钳制在 100% 的问题。',
				},
				{
					'en-US':
						'Modpack archives with GB18030 (GBK) encoded Chinese file names now extract correctly.',
					'zh-CN': '使用 GB18030（GBK）编码中文文件名的整合包压缩包现在可以正确解压。',
				},
			],
		},
	},
	{
		id: 'launcher-1.4.0',
		version: '1.4.0',
		publishedAt: '2026-07-23',
		title: {
			'en-US': 'Axolotl Launcher 1.4.0',
			'zh-CN': 'Axolotl Launcher 1.4.0',
		},
		changes: {
			added: [
				{
					'en-US':
						'Added categorized update announcements after app updates and a permanent release history in settings.',
					'zh-CN': '新增应用更新后的分类公告弹窗，以及设置中的永久版本历史记录。',
				},
				{
					'en-US': 'Added a first-run onboarding guide that can also be replayed from settings.',
					'zh-CN': '新增首次使用引导，并支持从设置中重新播放。',
				},
			],
			changed: [
				{
					'en-US': 'Skipped-download warnings can now be collapsed.',
					'zh-CN': '跳过下载模组的警告窗口现在可以被收起。',
				},
				{
					'en-US': 'Launcher logs now rotate automatically at 10 MiB and keep up to five files.',
					'zh-CN': '启动器日志现按 10 MiB 自动轮转并最多保留 5 个文件。',
				},
				{
					'en-US':
						'Modrinth request logs now retain the target, source, retry count, and a redacted URL.',
					'zh-CN': 'Modrinth 请求日志现在保留目标、来源、重试次数和脱敏 URL。',
				},
				{
					'en-US': 'Large error log exports now use streaming compression to reduce memory usage.',
					'zh-CN': '错误日志导出现在使用流式压缩，降低大日志导出时的内存占用。',
				},
				{
					'en-US':
						'WARN and ERROR logs now rotate before the 30 MiB boundary without splitting individual events.',
					'zh-CN': 'WARN 和 ERROR 日志现在会在 30 MiB 边界内保持完整，轮转时不会拆分单个事件。',
				},
				{
					'en-US': 'Launcher logs older than three days are now removed automatically.',
					'zh-CN': '启动器日志创建超过三天后现在会自动删除。',
				},
			],
			fixed: [
				{
					'en-US': 'Fixed skipped mods remaining in the list after manually installing them.',
					'zh-CN': '修复手动安装跳过下载的模组后，已跳过模组列表不会更新的问题。',
				},
				{
					'en-US':
						'Fixed duplicate download events causing complete installation states to be logged repeatedly.',
					'zh-CN': '修复下载事件重复记录完整安装状态，导致启动器日志快速膨胀的问题。',
				},
				{
					'en-US':
						'Fixed the Fabric/Modrinth content page watcher repeatedly writing the same map and getting stuck loading.',
					'zh-CN':
						'修复 Fabric/Modrinth 实例内容页 watcher 重复写入相同 Map，触发递归更新并持续加载的问题。',
				},
			],
			security: [
				{
					'en-US': 'Temporary signatures in Modrinth request URLs are no longer written to logs.',
					'zh-CN': 'Modrinth 请求 URL 中的临时签名不再写入日志。',
				},
			],
		},
	},
]

export function getAnnouncementByVersion(version: string | null | undefined) {
	if (!version) return undefined
	return launcherAnnouncements.find((announcement) => announcement.version === version)
}

export function getAnnouncements(): readonly LauncherAnnouncement[] {
	return launcherAnnouncements
}

export function getAnnouncementById(id: string) {
	return launcherAnnouncements.find((announcement) => announcement.id === id)
}

export function getLocalizedAnnouncementText(
	text: LocalizedAnnouncementText,
	locale: string,
): string {
	return locale === 'zh-CN' ? text['zh-CN'] : text['en-US']
}
