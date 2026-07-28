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
		id: 'launcher-1.6.0',
		version: '1.6.0',
		publishedAt: '2026-07-28',
		title: {
			'en-US': 'Axolotl Launcher 1.6.0',
			'zh-CN': 'Axolotl Launcher 1.6.0',
		},
		changes: {
			added: [
				{
					'en-US': 'Added Lab with a gradient color generator and a Java Edition Seed Map.',
					'zh-CN': '新增实验室，首批提供渐变颜色生成器和 Java 版种子地图。',
				},
			],
			changed: [
				{
					'en-US':
						'Improved download routing, retries, and progress reporting for more reliable installs.',
					'zh-CN': '优化下载源切换、重试与进度展示，提升安装下载的稳定性。',
				},
				{
					'en-US': 'Changed the way the launcher handles modpack parsing.',
					'zh-CN': '重写了加载器版本和类型的解析方式。',
				},
				{
					'en-US':
						'Changed some frontend code left by vibe and replaced it with native components.',
					'zh-CN': '重写了一些vibe留下的其它代码。',
				},
				{
					'en-US':
						'To avoid confusion caused by loaders that have not yet been parsed during batch imports, instances are now imported one by one with progress displayed.',
					'zh-CN': '为避免批量导入过程中还未来得及解析的加载器造成误解，现在逐个导入实例并显示进度',
				},
				{
					'en-US':
						'Improved the Linux desktop file (.desktop) with Comment, Keywords, StartupWMClass, and StartupNotify fields; added x-scheme-handler/axolotl protocol association and Chinese localization; and set WEBKIT_DISABLE_DMABUF_RENDERER=1 for Exec.',
					'zh-CN':
						'优化 Linux 桌面文件（.desktop）：补充 Comment、Keywords、StartupWMClass、StartupNotify 等字段，添加 x-scheme-handler/axolotl 协议关联与中文本地化，并为 Exec 添加 WEBKIT_DISABLE_DMABUF_RENDERER=1 环境变量。',
				},
				{
					'en-US':
						'Replaced Tauri template variables in the Linux desktop file template with fixed values, ensuring the built .desktop file uses "Axolotl Launcher" directly for its name, icon, and executable.',
					'zh-CN':
						'将 Linux 桌面文件模板从 Tauri 模板变量格式改为固定值格式，确保编译后的 .desktop 文件直接使用 "Axolotl Launcher" 作为名称、图标和可执行文件。',
				},
			],
			fixed: [
				{
					'en-US': 'Fixed the skin page failing to import skins.',
					'zh-CN': '修复了皮肤页面无法导入皮肤的问题。',
				},
				{
					'en-US': 'Fixed the import page failing to import instances.',
					'zh-CN': '修复了导入界面无法正常导入的bug。',
				},
			],
		},
	},
	{
		id: 'launcher-1.5.5',
		version: '1.5.5',
		publishedAt: '2026-07-26',
		title: {
			'en-US': 'Axolotl Launcher 1.5.5',
			'zh-CN': 'Axolotl Launcher 1.5.5',
		},
		changes: {
			added: [
				{
					'en-US':
						'The offline mode notice now has a refresh button to re-check the session server connection without restarting the launcher.',
					'zh-CN': '离线模式提示中新增刷新按钮，无需重启启动器即可重新检测会话服务器连接状态。',
				},
				{
					'en-US':
						'Interrupted downloads of large files now resume from where they left off instead of restarting from zero, including after switching download sources or retrying a failed install.',
					'zh-CN':
						'大文件下载中断后现在会从断点继续，而不是从头重新下载——切换下载源或重试失败的安装时同样生效。',
				},
				{
					'en-US':
						'Project pages now link to the matching MC Mod (mcmod.cn) wiki page — in the sidebar links and the top-right menu — when the project is found in the bundled wiki index. Works for both Modrinth and CurseForge projects.',
					'zh-CN':
						'项目详情页现在会链接到对应的 MC 百科（mcmod.cn）页面——位于侧栏相关链接和右上角菜单中，仅当项目能在内置百科索引中找到时显示。Modrinth 和 CurseForge 项目均支持。',
				},
			],
			changed: [
				{
					'en-US':
						"Checking a modpack's contents no longer loads the entire pack file into memory; it now streams to the download cache and is reused by a later install of the same version.",
					'zh-CN':
						'解析整合包内容时不再将整个整合包文件载入内存，而是流式下载到缓存，之后安装同一版本时可直接复用。',
				},
				{
					'en-US':
						'Leftover partial download files that have not been touched for a week are now cleaned up automatically on launch.',
					'zh-CN': '启动时会自动清理超过一周未使用的下载临时文件。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed a freeze caused by an infinite loop when closing the import method dialog, and its Cancel action is now a real button.',
					'zh-CN':
						'修复了关闭导入方式弹窗时因无限循环导致卡死的问题，同时「取消」现在是真正的按钮。',
				},
				{
					'en-US':
						'Forge, Fabric, and NeoForge files can now fall back to their official servers when download mirrors are unavailable or have not synced a newly released version yet.',
					'zh-CN':
						'当下载镜像不可用或尚未同步新发布的版本时，Forge、Fabric 和 NeoForge 文件现在会回退到官方服务器下载。',
				},
				{
					'en-US':
						'Servers that mishandle multi-connection downloads are now remembered during a session, so large files stop wasting a doomed segmented attempt before every download.',
					'zh-CN':
						'不支持多线程分段下载的服务器现在会在会话内被记住，大文件不再每次下载都先经历一轮注定失败的分段尝试。',
				},
				{
					'en-US':
						'Two downloads writing the same file at the same time can no longer corrupt each other’s temporary data.',
					'zh-CN': '同时写入同一文件的两个下载任务不再会相互破坏临时数据。',
				},
				{
					'en-US':
						'Importing an instance no longer shows a success notification before the import actually finishes — failures now report an error instead of a false success.',
					'zh-CN':
						'导入实例不再在导入真正完成前提示成功——导入失败时现在会提示错误，而不是错误地提示成功。',
				},
				{
					'en-US':
						'Changing the app directory now moves shared instance links without moving or copying their original files.',
					'zh-CN': '更改应用目录时，现在仅迁移共享实例链接，不再移动或复制其原始文件。',
				},
				{
					'en-US':
						'Creating a custom instance once again defaults its icon to the selected mod loader (Fabric, Forge, Quilt, NeoForge) instead of the generic placeholder.',
					'zh-CN':
						'创建自定义实例时，图标重新默认使用所选加载器的图标（Fabric、Forge、Quilt、NeoForge），不再是通用占位图。',
				},
				{
					'en-US':
						'Loader and other newer built-in instance icons now display without the avatar frame, matching the rest of the built-in icons.',
					'zh-CN': '加载器及其他较新的内置实例图标现在与其余内置图标一致，不再带边框显示。',
				},
				{
					'en-US':
						'Fixed the launcher failing to start with a "Cannot save an incomplete Java installation" error when a leftover unfinished Java download was found while changing the app directory or migrating old launcher data.',
					'zh-CN':
						'修复更改应用目录或迁移旧启动器数据时，遗留的未完成 Java 下载会导致启动器无法启动并报 "Cannot save an incomplete Java installation" 错误的问题。',
				},
			],
		},
	},
	{
		id: 'launcher-1.5.4',
		version: '1.5.4',
		publishedAt: '2026-07-25',
		title: {
			'en-US': 'Axolotl Launcher 1.5.4',
			'zh-CN': 'Axolotl Launcher 1.5.4',
		},
		changes: {
			added: [
				{
					'en-US':
						'Added a transparent background option in Settings > Appearance, with a slider to control how much of your desktop shows through the launcher window.',
					'zh-CN': '设置 > 外观新增「透明背景」选项，可通过滑块调节桌面透过启动器窗口显示的程度。',
				},
				{
					'en-US':
						'Added a background blur toggle for the transparent background, frosting whatever shows through the window.',
					'zh-CN': '透明背景新增「背景模糊」开关，可将透出的画面做磨砂玻璃处理。',
				},
				{
					'en-US': 'Added powerful modpack parsing functionality.',
					'zh-CN': '整合包强力解析功能',
				},
				{
					'en-US': 'Automatically set instance icons to match their mod loader.',
					'zh-CN': '自动设置实例图标为加载器图标。',
				},
			],
			fixed: [
				{
					'en-US': 'Fixed frontend display errors during modpack import.',
					'zh-CN': '修复整合包导入时的前端显示错误',
				},
			],
		},
	},
	{
		id: 'launcher-1.5.3',
		version: '1.5.3',
		publishedAt: '2026-07-25',
		title: {
			'en-US': 'Axolotl Launcher 1.5.3',
			'zh-CN': 'Axolotl Launcher 1.5.3',
		},
		changes: {
			added: [
				{
					'en-US':
						'Added translation for new entries, allowing the translation feature to be applied to titles and descriptions outside of entries.',
					'zh-CN': '新增条目翻译功能，让翻译功能可以应用到条目外的标题和介绍。',
				},
			],
			fixed: [
				{
					'en-US': 'Urgent fix for critical bugs in the previous version',
					'zh-CN': '紧急修复上个版本严重bug',
				},
				{
					'en-US':
						'Transient Windows file locks are now retried during downloads, and persistent lock errors identify the process holding the file when Windows can report it.',
					'zh-CN':
						'下载时遇到短暂的 Windows 文件占用将自动重试；若持续失败,Windows 能识别时会在错误中显示占用文件的进程。',
				},
			],
			changed: [
				{
					'en-US':
						'Changed the way the module loader is recognized when importing instances, using a more aggressive strategy',
					'zh-CN': '更改导入实例时模组加载器的识别方式,采用更激进的策略。',
				},
				{
					'en-US':
						'Changed the way the import type is detected, using a more conservative strategy',
					'zh-CN': '更改导入类型探测的方式,采用更保守的策略。',
				},
				{
					'en-US': 'Changed some frontend code left by vibe and replaced it with native components',
					'zh-CN': '修改了一些曾经vibe留下的前端代码,换为原生组件。',
				},
				{
					'en-US':
						'Changed the scanning logic to optimize some parts of the import scanning, improving compatibility.',
					'zh-CN': '修改扫描逻辑，优化导入扫描的部分石山，提升兼容性。',
				},
			],
		},
	},
	{
		id: 'launcher-1.5.2',
		version: '1.5.2',
		publishedAt: '2026-07-25',
		title: {
			'en-US': 'Axolotl Launcher 1.5.2',
			'zh-CN': 'Axolotl Launcher 1.5.2',
		},
		changes: {
			added: [
				{
					'en-US':
						'Drag and drop mods, resource packs, shader packs, world saves, schematic files, and launcher instances anywhere in the launcher for instant import — no need to navigate menus.',
					'zh-CN':
						'新增全局拖拽功能：直接拖入模组、资源包、光影包、存档、投影文件及启动器，即可快速导入，无需在菜单中翻找。',
				},
				{
					'en-US':
						'Added schematic file management — import and manage .schematic and .litematica files alongside your mods and worlds.',
					'zh-CN': '新增原理图管理：支持导入和管理 .schematic 及 .litematica 格式的结构投影文件。',
				},
				{
					'en-US':
						'Added mod import validation — when installing a mod, the launcher now checks if it is compatible with your current Minecraft version and mod loader, and warns you before installing if something does not match.',
					'zh-CN':
						'新增模组导入校验：安装模组时，启动器会自动检测其与当前 Minecraft 版本和加载器的兼容性，不匹配时会提前提醒。',
				},
				{
					'en-US':
						'Added mod metadata parsing — the launcher can now read mod name, version, supported loader, and other details directly from mod files.',
					'zh-CN':
						'新增 Mod 文件元数据解析：启动器可直接从模组文件中读取名称、版本、适用加载器等信息。',
				},
				{
					'en-US':
						'Installed mods in the instance content tab and the modpack content dialog now show bilingual "中文名 (English)" titles under the Simplified Chinese locale, and installed content can be searched in Chinese.',
					'zh-CN':
						'中文界面下，实例内容页与整合包内容弹窗的已装模组现以「中文名 (英文名)」显示，并支持用中文搜索已装内容。',
				},
				{
					'en-US':
						'Under the Simplified Chinese locale, newly downloaded mods, resource packs, shader packs and data packs are saved as "[中文名]original-name" when a Chinese name is known; unknown files keep their original names and exported modpacks always restore the original file names.',
					'zh-CN':
						'中文界面下，新下载的模组、资源包、光影包和数据包会以「[中文名]原文件名」保存；查不到中文名时保持原样，导出整合包时自动还原为原文件名。',
				},
				{
					'en-US':
						'Browsing the Discover Content page without searching now also shows bilingual "中文名 (English)" titles under the Simplified Chinese locale, for both Modrinth and CurseForge results.',
					'zh-CN':
						'中文界面下，「发现内容」页直接浏览（不搜索）时也会显示「中文名 (英文名)」双语标题，Modrinth 与 CurseForge 结果均生效。',
				},
				{
					'en-US':
						'The game language now follows the launcher language on the first launch of an instance, including imported modpacks, using the correct language code for each game version; instances you already play keep your in-game choice.',
					'zh-CN':
						'游戏语言现在会在实例首次启动时自动跟随启动器语言（包括导入的整合包），并按游戏版本写入正确的语言代码；已游玩过的实例仍保留游戏内的语言设置。',
				},
				{
					'en-US':
						'The left sidebar now animates the active highlight sliding between pages when switching sections, matching the content type tabs.',
					'zh-CN': '左侧导航栏切换页面时，选中高亮改为滑动过渡动画，与顶部内容类型标签栏保持一致。',
				},
				{
					'en-US':
						'You can now write a custom system prompt for OpenAI-compatible translation services (Settings > Translation).',
					'zh-CN': '现在可以在翻译设置中为 OpenAI 兼容服务编写自定义系统提示词。',
				},
				{
					'en-US':
						'Translation results now appear in staggered batches with a smooth floating animation.',
					'zh-CN': '翻译结果现在以逐批浮动动画显示，视觉体验更流畅。',
				},
				{
					'en-US':
						'Added a Windows option to use the high-performance GPU for the launcher and Java.',
					'zh-CN': '新增 Windows 高性能显卡选项，可用于启动器和 Java。',
				},
				{
					'en-US': 'Added local Minecraft crash diagnosis and exportable diagnostic reports.',
					'zh-CN': '新增本地 Minecraft 崩溃诊断和可导出的诊断报告。',
				},
				{
					'en-US':
						'Legacy (1.14 and below), April fools and snapshot versions of Minecraft can now be installed through instance creation.',
					'zh-CN': '现在可以通过创建实例安装 Minecraft 的旧版（1.14及以下）、愚人节版和快照版。',
				},
				{
					'en-US': 'Forge, NeoForge, Fabric and Quilt icons will now be auto set.',
					'zh-CN': 'Forge、NeoForge、Fabric 和 Quilt 的图标现在会自动设置。',
				},
			],
			changed: [
				{
					'en-US':
						'Improved modpack import compatibility — more modpack formats are supported and edge cases are handled better, so more modpacks import successfully.',
					'zh-CN':
						'优化整合包导入兼容性：支持更多整合包格式，能更好地处理各种特殊情况，导入成功率更高。',
				},
				{
					'en-US':
						'Improved mod import compatibility — better detection and handling of different mod file types during the import process.',
					'zh-CN': '优化模组导入兼容性：导入时能更准确地识别和处理不同类型的模组文件。',
				},
				{
					'en-US':
						'Java detection is now faster: it reads a metadata file in each installation to determine the version instead of launching a JVM for every candidate, reducing the delay of the first system scan.',
					'zh-CN':
						'加快 Java 检测：现在优先读取每个安装目录的元数据文件判断版本，避免为每个候选启动 JVM，减少首次扫描的耗时。',
				},
				{
					'en-US':
						'Downloading or launching an instance now scans the system for an already-installed Java of the required version before downloading a new runtime, reusing an existing installation instead of downloading a duplicate.',
					'zh-CN':
						'下载或启动实例时，现在会先扫描本机是否已安装所需版本的 Java，找到则复用，仅在确实没有时才下载新的运行时，避免重复下载。',
				},
				{
					'en-US':
						'Crash diagnostics now combine related logs and provide direct analysis and export actions.',
					'zh-CN': '崩溃诊断现在会归集相关日志，并提供直接分析和导出操作。',
				},
				{
					'en-US':
						'The log console and local crash diagnosis are now fully localized in English, Simplified Chinese, and Traditional Chinese.',
					'zh-CN': '日志控制台与本地崩溃诊断现已完整支持英语、简体中文和繁体中文。',
				},
				{
					'en-US':
						'Empty log consoles now show Chinese startup guidance with a pink side-view axolotl illustration matching the launcher icon.',
					'zh-CN': '空日志控制台现在会显示中文启动提示，以及贴近启动器图标的粉色美西螈侧视字符画。',
				},
				{
					'en-US':
						'Translation requests are now sent in batches (5 segments per batch) to reduce API overhead.',
					'zh-CN': '翻译请求现在分批发送（每批5个段落），降低 API 调用频率。',
				},
				{
					'en-US':
						'Offline account creation now warns when a Chinese username may be incompatible with Minecraft 1.18 and newer.',
					'zh-CN':
						'创建离线账户时，若使用中文用户名，现在会提示其可能与 Minecraft 1.18 及以上版本不兼容。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed some account avatars appearing blank after the launcher starts until the account is selected.',
					'zh-CN': '修复启动器启动后部分账号头像显示空白、需要切换账号才恢复的问题。',
				},
				{
					'en-US':
						'Improved large-file download throughput with parallel Range requests, safer retries, and redirect reuse.',
					'zh-CN': '通过并行 Range 请求、安全重试和重定向复用提升大文件下载速度。',
				},
				{
					'en-US':
						'Fixed startup failures caused by conflicting Java discovery and onboarding database migrations.',
					'zh-CN': '修复 Java 检测与新手引导数据库迁移冲突导致的启动失败。',
				},
				{
					'en-US':
						'Fixed the accent highlight outline on the Add skin button in the skin selector being clipped on some edges when the button was focused.',
					'zh-CN':
						'修复皮肤选择器「添加皮肤」按钮在聚焦时强调色高亮描边部分边缘被裁剪、显示不完整的问题。',
				},
				{
					'en-US':
						"Fixed database backups being written to Modrinth's directory; backups are now stored in the launcher's own data directory.",
					'zh-CN':
						'修复数据库备份被写入 Modrinth 目录的问题，现在改为保存到启动器自己的应用数据目录。',
				},
				{
					'en-US': 'Improved crash diagnosis when multiple instances fail close together.',
					'zh-CN': '改进多个实例接连失败时的崩溃诊断。',
				},
				{
					'en-US': 'Fixed early Java and loader failures leaving instances stuck while starting.',
					'zh-CN': '修复 Java 或加载器早期失败时实例持续卡在启动中的问题。',
				},
			],
		},
	},
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
				{
					'en-US':
						'Added automatic memory allocation that adapts to available RAM and installed mods each time an instance launches.',
					'zh-CN': '新增自动分配内存，可在每次启动实例时根据可用内存和已安装模组动态调整。',
				},
				{
					'en-US':
						'Added a live memory allocation display and one-click memory optimization on Windows.',
					'zh-CN': '新增实时内存分配展示，并在 Windows 上提供一键内存优化。',
				},
			],
			changed: [
				{
					'en-US':
						'Java detection now caches results, scans sources concurrently, and refreshes the installation list in the background.',
					'zh-CN': 'Java 检测现在会缓存结果、并行扫描不同来源，并在后台刷新安装列表。',
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
					'en-US': 'Improved memory usage reporting and automatic allocation accuracy on macOS.',
					'zh-CN': '改进 macOS 上的内存占用显示和自动分配准确性。',
				},
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
					'zh-CN': '新增 HMCL、PCL2、PCL2CE 启动器实例导入支持，可直接根据启动器解析出所有实例。',
				},
				{
					'en-US':
						'Added generic folder import — any directory containing a .minecraft folder can now be imported as an instance.',
					'zh-CN': '新增通用文件夹导入功能，可导入任意含 .minecraft 的目录。',
				},
				{
					'en-US':
						'Added "import as shared instance" support, optionally using symlinks instead of copying to save disk space.',
					'zh-CN': '新增添加为共享实例功能：导入时可选软链接而非复制。',
				},
				{
					'en-US': 'Added a confirmation dialog when deleting files from the file browser tab.',
					'zh-CN': '补齐文件标签页删除时的确认弹窗。',
				},
				{
					'en-US':
						'Added OptiFine support — declared OptiFine in a modpack is automatically installed; standalone, or as a mod alongside other loaders.',
					'zh-CN': '新增 OptiFine 支持：整合包声明 OptiFine 时自动安装——单独存在时作为加载器。',
				},
				{
					'en-US':
						'Added drag-and-drop import: drop mods, resource packs, shader packs, world saves, schematics, and launcher instances directly onto the launcher for instant import.',
					'zh-CN':
						'新增拖放导入功能：直接拖入模组、资源包、光影包、存档、投影文件及启动器实例，即可快速导入。',
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
					'en-US': 'Updated shared instance indicators and warning hints for clarity.',
					'zh-CN': '更新共享实例标识与警告提示。',
				},
				{
					'en-US':
						'Greatly improved modpack import compatibility — now handles CurseForge, MCBBS, HMCL, MultiMC, PCL launcher-bundled archives and various non-standard pack formats.',
					'zh-CN':
						'大大增强整合包导入兼容性，兼容 CurseForge、MCBBS、HMCL、MultiMC、PCL 等导出的附带启动器的整合包以及各种不完全符合规范的整合包格式。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed world save import failing with "Invalid instance ID" error due to incorrect UUID parsing of local instance IDs.',
					'zh-CN':
						'修复世界存档导入时因实例 ID 的 local: 前缀被错误地当作 UUID 解析而导致的导入失败问题。',
				},
				{
					'en-US':
						'Fixed "[object Object]" being displayed in error notifications instead of the actual error message.',
					'zh-CN': '修复错误通知中显示 "[object Object]" 而非真实错误信息的问题。',
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
					'en-US':
						'Improved the update settings version history with clearer release cards and details.',
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
