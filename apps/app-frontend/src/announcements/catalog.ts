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
		id: 'launcher-1.8.1',
		version: '1.8.1',
		publishedAt: '2026-08-16',
		title: {
			'en-US': 'Axolotl Launcher 1.8.1',
			'zh-CN': 'Axolotl Launcher 1.8.1',
		},
		changes: {
			added: [
				{
					'en-US':
						'Discover now remembers the last selected content source (Modrinth, CurseForge, or all sources), reopening on the previously used platform instead of resetting.',
					'zh-CN':
						'“发现内容”板块现在会记住上次选择的模组来源（Modrinth、CurseForge 或所有来源），再次打开时恢复上次使用的平台，不再重置为所有来源。',
				},
				{
					'en-US':
						'Screenshots in the instance screenshots page now load through optimized thumbnails, so large screenshot libraries open faster and use less memory.',
					'zh-CN': '实例截图页现在会加载优化后的缩略图，截图较多时打开更快、占用内存更低。',
				},
				{
					'en-US':
						'CurseForge install previews now flag dependency versions that may not match the target instance, so incompatible fallback selections are visible before installing.',
					'zh-CN':
						'CurseForge 安装预览现在会标记可能与目标实例版本不匹配的依赖版本，在安装前即可看到不兼容的兜底选择。',
				},
				{
					'en-US':
						'Added a gallery layout to the Discover page for browsing content with visual thumbnails.',
					'zh-CN': '发现内容页新增画廊布局，支持以缩略图形式浏览内容。',
				},
			],
			changed: [
				{
					'en-US': 'Optimized the category menu in settings.',
					'zh-CN': '优化了设置中的分类菜单。',
				},
				{
					'en-US': 'Redesigned the "About" page.',
					'zh-CN': '重新设计了“关于”页面。',
				},
				{
					'en-US': 'Optimized dependency-related handling logic.',
					'zh-CN': '优化依赖相关处理逻辑。',
				},
				{
					'en-US':
						'CurseForge managed modpack version switching now runs as a tracked download task like Modrinth updates, with real progress, success and failure notifications, and automatic resume after manually downloaded files are imported.',
					'zh-CN':
						'CurseForge 整合包切换版本现在会像 Modrinth 更新一样作为可跟踪的下载任务运行，显示真实进度与成功/失败通知，并可在手动下载的文件补齐后自动继续。',
				},
				{
					'en-US':
						'Switched Fabric, Forge, NeoForge, and Quilt version metadata to official sources with Modrinth fallback, providing more complete and up-to-date loader version lists.',
					'zh-CN':
						'Fabric、Forge、NeoForge 和 Quilt 的版本元数据现优先从官方源获取，并在必要时回退至 Modrinth，提供更完整、更及时的加载器版本列表。',
				},
				{
					'en-US':
						'Improved loader version compatibility across instance creation, settings, repair, offline launch, and imported instances, preserving exact loader versions and preventing unintended fallback to newer releases.',
					'zh-CN':
						'改进了实例创建、设置、修复、离线启动和实例导入中的加载器版本兼容性，保留精确的加载器版本，并避免意外回退到更新版本。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed CurseForge modpack version switching appearing to do nothing after confirmation; errors are now reported and the switch is tracked in the download manager.',
					'zh-CN':
						'修复 CurseForge 整合包确认切换版本后看起来毫无反应的问题；现在会显示错误信息，并在下载管理器中跟踪切换进度。',
				},
				{
					'en-US':
						'Fixed CurseForge modpack update progress staying stuck while the launcher calibrated the pack before downloading.',
					'zh-CN': '修复 CurseForge 整合包更新在下载前校准阶段进度一直不动的问题。',
				},
				{
					'en-US':
						'Fixed CurseForge modpack version lists only showing the latest 50 files; all published versions now appear and the currently installed version is always included.',
					'zh-CN':
						'修复 CurseForge 整合包版本列表只显示最新 50 个文件的问题；现在会显示全部已发布版本，并始终包含当前安装的版本。',
				},
				{
					'en-US': 'Fixed modal close buttons shifting out of place in merged-header dialogs.',
					'zh-CN': '修复合并式标题栏弹窗中关闭按钮位置偏移的问题。',
				},
				{
					'en-US':
						'Fixed day buttons in the home page calendar widget being partially obscured by overlapping elements.',
					'zh-CN': '修复主页日历组件内天数按钮会被遮挡的问题。',
				},
				{
					'en-US':
						'Fixed truncated titles faintly visible in the Discover page, improving text clipping and layout boundaries.',
					'zh-CN': '修复发现内容页隐约能看到截断的标题的问题，优化了文本裁剪和布局边界。',
				},
				{
					'en-US':
						'Fixed loader tags not displaying on the Discover page, so mod loader labels now appear correctly.',
					'zh-CN': '修复发现内容页不显示加载器标签的问题，现在模组加载器标签可以正常显示。',
				},
			],
		},
	},
	{
		id: 'launcher-1.8.0',
		version: '1.8.0',
		publishedAt: '2026-08-15',
		title: {
			'en-US': 'Axolotl Launcher 1.8.0',
			'zh-CN': 'Axolotl Launcher 1.8.0',
		},
		changes: {
			added: [
				{
					'en-US':
						'Added logshare.cn log sharing and AI log analysis features, and optimized the log viewer page.',
					'zh-CN': '新增logshare.cn的日志分享和AI日志分析功能，优化了日志查看器页面。',
				},
				{
					'en-US':
						'Added automatic recovery for interrupted CurseForge manual-download installations, allowing incomplete modpack installs to safely continue after restarting the launcher.',
					'zh-CN':
						'新增 CurseForge 手动下载安装任务的自动恢复机制，启动器重启后也能安全恢复并继续未完成的整合包安装。',
				},
				{
					'en-US':
						'Added instance content filtering, allowing users to filter instance content by type, source, and other metadata.',
					'zh-CN': '新增实例内容筛选功能。',
				},
				{
					'en-US': 'Added data pack management features.',
					'zh-CN': '新增数据包管理功能。',
				},
				{
					'en-US': 'Datapacks exported from the recipe generator now have descriptions and icons.',
					'zh-CN': '数据包配方生成器导出的数据包现在有了描述和图标。',
				},
				{
					'en-US': 'Added telemetry features.',
					'zh-CN': '新增遥测功能',
				},
			],
			changed: [
				{
					'en-US':
						'Changed the sidebar collapse method and optimized the sidebar collapse and expand interaction.',
					'zh-CN': '修改侧边栏折叠方式，优化了侧边栏的收起展开交互。',
				},
				{
					'en-US': 'Removed duplicate options in the content interface, such as the delete button.',
					'zh-CN': '移除内容界面部分重复的选项，例如删除按钮。',
				},
				{
					'en-US':
						'Added a delete button to the schematic management interface and moved the edit button to a more organized position.',
					'zh-CN': '为投影管理增加删除按钮，移动编辑按钮到整齐的位置。',
				},
				{
					'en-US':
						'Improved the CurseForge missing-file recovery workflow. Manual imports, monitored-folder imports, and the missing-files dialog now use the latest pending state to stay synchronized.',
					'zh-CN':
						'优化 CurseForge 缺失文件补全流程，手动导入、监听文件夹自动导入与缺失文件窗口现在会基于最新待处理状态保持同步。',
				},
				{
					'en-US':
						'Improved matching for browser-downloaded CurseForge files, including localized file names with title prefixes while retaining strict file verification.',
					'zh-CN':
						'优化浏览器下载的 CurseForge 文件识别，现在支持带有本地化标题前缀的文件名，同时仍保持严格的文件校验。',
				},
				{
					'en-US':
						'Strengthened CurseForge manual-file verification and import handling to better tolerate concurrent automatic and manual imports without duplicating or incorrectly accepting files.',
					'zh-CN':
						'加强 CurseForge 手动文件的校验与导入处理，在自动导入与手动导入同时发生时也能更可靠地避免重复导入或错误接受文件。',
				},
				{
					'en-US':
						'Improved the recent projects widget layout: added 3x1 and 3x2 sizes, and hid game mode labels and the metadata separator dot in the 2x1 and 2x2 sizes.',
					'zh-CN':
						'改进“从最近的项目开始”组件布局：新增 3x1 和 3x2 尺寸，并在 2x1 和 2x2 尺寸下隐藏游戏模式及分隔小圆点。',
				},
				{
					'zh-CN': '补齐了部分本地化翻译',
					'en-US': 'Completed some localizations',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed cases where an already imported CurseForge file could still be reported as not pending or shown as a verification failure because of stale recovery state.',
					'zh-CN':
						'修复 CurseForge 文件已经成功导入后，因恢复状态过期仍提示“文件不在待处理列表中”或错误显示为校验失败的问题。',
				},
				{
					'en-US':
						'Fixed the color picker timing issue in the Gradient Text Generator in the Labs section.',
					'zh-CN': '修复实验室中“渐变文字生成器”的取色器取色时机问题。',
				},
				{
					'en-US':
						'Fixed stale missing-file records reappearing after refreshing or reopening the missing-content workflow.',
					'zh-CN':
						'修复刷新或重新打开缺失内容补全流程后，已经处理完成的缺失文件记录可能再次出现的问题。',
				},
				{
					'en-US':
						'Fixed ordinary content installations incorrectly marking an unfinished modpack instance as fully installed while the original installation was still waiting for missing files.',
					'zh-CN':
						'修复整合包仍在等待缺失文件时，单独安装其他内容可能错误地将整个实例标记为安装完成的问题。',
				},
				{
					'en-US':
						'Fixed install progress leaking between different installation phases, which could cause downloaded bytes from an earlier stage to appear in a later stage.',
					'zh-CN':
						'修复不同安装阶段之间的进度数据串用问题，避免上一阶段的已下载大小被错误带入后续阶段。',
				},
				{
					'en-US':
						'Fixed the download page showing historical download totals as current progress during non-download phases such as resolving the mod loader.',
					'zh-CN': '修复下载页面在解析加载器等非下载阶段仍将历史下载统计显示为当前进度的问题。',
				},
				{
					'en-US':
						'Fixed incorrect download progress displays where the downloaded size could temporarily exceed the total size after skipping or recovering missing content.',
					'zh-CN':
						'修复跳过或补全缺失内容后，下载进度可能短暂出现“已下载大小大于总大小”的异常显示。',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.7',
		version: '1.7.7',
		publishedAt: '2026-08-14',
		title: {
			'en-US': 'Axolotl Launcher 1.7.7',
			'zh-CN': 'Axolotl Launcher 1.7.7',
		},
		changes: {
			added: [
				{
					'en-US': 'Emergency fix for DB migration crash.',
					'zh-CN': '紧急修复DB migration崩溃。',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.6',
		version: '1.7.6',
		publishedAt: '2026-08-14',
		title: {
			'en-US': 'Axolotl Launcher 1.7.6',
			'zh-CN': 'Axolotl Launcher 1.7.6',
		},
		changes: {
			added: [
				{
					'en-US':
						'Added a preferred IP solution for domestic use of Google Translate, now Google Translate can be used in domestic network environments.',
					'zh-CN':
						'为Google翻译的国内使用提供了优选IP方案，现在可以在国内网络环境使用Google翻译了。',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.5',
		version: '1.7.5',
		publishedAt: '2026-08-13',
		title: {
			'en-US': 'Axolotl Launcher 1.7.5',
			'zh-CN': 'Axolotl Launcher 1.7.5',
		},
		changes: {
			fixed: [
				{
					'en-US':
						'Fixed unpredictable account ordering when multiple accounts share the same name and login type.',
					'zh-CN': '修复了多个同名同类型账号顺序不固定的问题。',
				},
				{
					'en-US':
						'Fixed the issue of CurseForge modpacks not showing updates after the latest version was published.',
					'zh-CN': '修复CurseForge整合包错误显示更新的问题。',
				},
				{
					'en-US': 'Fixed the issue of CurseForge resources failing to parse correctly.',
					'zh-CN': '修复CurseForge资源解析错误问题。',
				},
			],
			changed: [
				{
					'en-US':
						'Improved the readability of the duplicate custom UUID error message in the offline account dialog.',
					'zh-CN': '优化了离线账户弹窗中重复自定义 UUID 的错误提示可读性。',
				},
				{
					'en-US':
						'Optimized the filtering function of the instance interface content section, now allowing multiple filter conditions to be selected by holding down ctrl.',
					'zh-CN': '优化实例界面内容板块的筛选功能，现在按住ctrl可多选筛选条件。',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.4',
		version: '1.7.4',
		publishedAt: '2026-08-12',
		title: {
			'en-US': 'Axolotl Launcher 1.7.4',
			'zh-CN': 'Axolotl Launcher 1.7.4',
		},
		changes: {
			changed: [
				{
					'en-US': 'Changed the implementation of automatic switching of instance icons.',
					'zh-CN': '更改了自动切换实例图标的实现方式。',
				},
				{
					'en-US': 'Refactored the interface for importing instances.',
					'zh-CN': '全新的导入实例的界面，可以自定义更多选项',
				},
				{
					'en-US':
						'Further optimized the download strategy and process to reduce the probability of download failures.',
					'zh-CN': '进一步优化下载策略与流程。',
				},
				{
					'en-US':
						'Optimized the recipe generator page, changing the list to lazy loading to improve performance.',
					'zh-CN': '优化配方生成器页面，列表改为懒加载，提升性能。',
				},
			],
			added: [
				{
					'en-US': 'Added RedStone multiplayer for creating rooms that friends can join directly.',
					'zh-CN': '新增了红石联机，可以让你的好友直接加入你的房间。',
				},
				{
					'en-US': 'Added a settings for enabling/disabling page transitions.',
					'zh-CN': '新增启动器设置项, 可开启/关闭页面切换动画。',
				},
				{
					'en-US':
						'Added detailed Multiplayer diagnostics and exportable error reports for troubleshooting connection failures.',
					'zh-CN': '联机页面新增详细诊断日志与错误报告导出, 便于排查连接失败。',
				},
				{
					'en-US':
						'Added a setting for configuring the Mojang authentication service mirror for all possible locations.',
					'zh-CN': '为所有可能的地方配置了Mojang service换源设置。',
				},
				{
					'en-US': 'Added support for automatic import of datapacks.',
					'zh-CN': '添加对数据包的自动导入支持。',
				},
				{
					'en-US':
						'Added a mechanism to reuse existing resources when importing external instances, now avoiding some duplicate downloads.',
					'zh-CN': '添加了在导入外部实例时复用已有资源的机制，现在避免一部分重复下载。',
				},
				{
					'en-US':
						'Added a targeted project-cache repair and retry action for recoverable CurseForge metadata cache failures.',
					'zh-CN': '为可恢复的 CurseForge 项目元数据缓存故障新增定向清理缓存并重试操作。',
				},
				{
					'en-US': 'Added custom public node configuration for Terracotta multiplayer.',
					'zh-CN': '新增陶瓦联机自定义公共节点配置。',
				},
				{
					'en-US':
						'Now the launcher allows users to skip missing files when importing, and import later.',
					'zh-CN': '现在允许用户在遇到缺失文件时选择跳过，后续再导入。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed corrupted launcher databases repeatedly showing errors when opening instance content; Axolotl now restores the latest verified backup while preserving the damaged database files.',
					'zh-CN':
						'修复启动器数据库损坏后，打开实例内容页会反复报错的问题；现在会从最新且校验正常的备份恢复，并保留损坏的数据库文件。',
				},
				{
					'en-US':
						'Fixed external Minecraft instance imports occasionally failing with an "i/o error" when duplicate native libraries were extracted concurrently.',
					'zh-CN':
						'修复同时导入外部 Minecraft 实例时，重复原生库并发解压可能导致“i/o error”并使导入失败的问题。',
				},
				{
					'en-US':
						'Refactored the recognition logic when importing resources, fixing the issue of incorrect root directories when importing nested resources.',
					'zh-CN': '重构了导入资源时的识别逻辑，修复了导入资源嵌套时根目录错误的问题。',
				},
				{
					'en-US': 'Fixed the laggy transition animation.',
					'zh-CN': '优化了页面切换动画的卡顿问题。',
				},
				{
					'en-US': 'Fixed the issue of the loading bar not extending to full screen.',
					'zh-CN': '修复了加载条未能延伸至全屏的问题。',
				},
				{
					'en-US':
						'Fixed launch being incorrectly blocked when the custom Java did not match the recommended version; the user-selected Java is now used, with fallback to a compatible runtime when the recommended one is missing.',
					'zh-CN':
						'修复自定义 Java 与推荐版本不一致时误阻止启动的问题，现优先使用用户指定 Java，缺失推荐版本时回退至兼容版本。',
				},
				{
					'en-US':
						'Fixed the issue of the launcher not being able to start when the system language is set to Chinese.',
					'zh-CN':
						'针对Github Copilot auto模型做了特殊处理, 现在Github Copilot 免费套餐也可以使用Auto模型。',
				},
				{
					'en-US':
						'Fixed the issue of the launcher triggering page transitions when switching tabs within a page.',
					'zh-CN': '修复在页面内切换标签分类时, 启动器会错误触发页面切换动画的问题。',
				},
				{
					'en-US':
						'Fixed project platform and tag links not returning to Discover Content with the matching filter applied.',
					'zh-CN': '修复项目详情页的平台与标签链接无法返回发现内容页并应用对应筛选的问题。',
				},
				{
					'en-US':
						'Fixed invalid Multiplayer room codes showing raw internal errors instead of a localized format hint.',
					'zh-CN': '修复联机房间码格式错误时显示内部原始报错的问题, 现改为本地化格式提示。',
				},
				{
					'en-US':
						'en-US locales modification to avoid translation overflow in buttons. (issue #212)',
					'zh-CN': '修复了按钮中的英语翻译过长导致按钮文案溢出的问题。(issue #212)',
				},
				{
					'en-US': 'Removed the weird padding of home layout switch (issue #210)',
					'zh-CN': '移除了主页布局切换按钮的奇怪内边距（issue #210）',
				},
				{
					'en-US':
						'Fixed: Buttons would be overlapping in the schematic preview edit panel (issue #212)',
					'zh-CN': '修复了在投影预览编辑面板中按钮重叠的问题。(issue #212)',
				},
				{
					'en-US': 'Fixed overflow of directory in the warning of deleting symlink instances',
					'zh-CN': '修复了删除符号链接实例的警告中提示的目录溢出的问题',
				},
				{
					'en-US': 'Fixed the broken live log terminal',
					'zh-CN': '修复了实时日志无法正常更新的问题',
				},
				{
					'en-US':
						'When launcher gets a conflicting cache alias from database, it will now resolve cache conflicts automatically, instead of throwing an error.',
					'zh-CN':
						'当启动器从数据库获取到冲突的缓存别名时，现在会自动解决缓存冲突，而不是抛出错误。',
				},
				{
					'en-US':
						'Fixed modpack installations failing after remote project metadata was fetched but could not be written to the local cache.',
					'zh-CN': '修复远程项目元数据已获取成功、但无法写入本地缓存时导致整合包安装失败的问题。',
				},
				{
					'en-US':
						'Fixed instances being shown as ready before installation fully completed, which could trigger incorrect refreshes after a failed install.',
					'zh-CN': '修复实例在安装完全结束前就显示为可用，导致安装失败后仍可能错误刷新的问题。',
				},
				{
					'en-US':
						'Fixed canceling a modpack install during override extraction leaving the new instance and partially installed files behind instead of completing rollback.',
					'zh-CN':
						'修复在解压整合包覆盖文件时取消安装，回滚未完成并残留新实例及部分安装文件的问题。',
				},
				{
					'en-US': 'Fixed the issue of the recipe generator crashing in some cases.',
					'zh-CN': '修复配方生成器中部分情况崩溃的问题。',
				},
				{
					'en-US':
						'Fixed the issue of front-end performance blocking when downloading too many small files.',
					'zh-CN': '修复在下载小文件过多时造成的前端性能阻塞问题。',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.3',
		version: '1.7.3',
		publishedAt: '2026-08-10',
		title: {
			'en-US': 'Axolotl Launcher 1.7.3',
			'zh-CN': 'Axolotl Launcher 1.7.3',
		},
		changes: {
			added: [
				{
					'en-US': 'Added animation for switching launcher pages',
					'zh-CN': '为启动器页面切换添加了动画',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed cache purging failing when the frontend sends cache types unsupported by the running backend.',
					'zh-CN': '修复前端发送当前后端不支持的缓存类型时, 清理缓存失败的问题。',
				},
				{
					'en-US': 'Fixed Curseforge import missing files issues',
					'zh-CN': '修复 Curseforge 导入缺失文件问题',
				},
				{
					'en-US':
						'Fixed the issue when after manually added missing files, the download indicator will wrongly show a downloaded size larger than excepted size.',
					'zh-CN': '修复手动添加缺失文件后, 下载指示器错误显示已下载文件大小大于预期大小的问题',
				},
				{
					'en-US':
						'Fixed the issue when Discord Rich Presence is not reachable, Minecraft can not start properly.',
					'zh-CN': '修复当 Discord Rich Presence 不可用（超时）时, Minecraft 无法正常启动的问题。',
				},
				{
					'en-US':
						'Fixed nsis installer not installing the launcher into the correct specified directory.',
					'zh-CN': '修复 nsis 安装程序未能将启动器安装到指定目录的问题。',
				},
				{
					'en-US': 'Fixed Minecraft account name fetch blocking splash screen from closing.',
					'zh-CN': '修复 Minecraft 账号名称获取堵塞导致阻止启动器Splash screen关闭的问题。',
				},
			],
			changed: [
				{
					'en-US':
						'When changing theme color, the shadow color of the launcher now changes accordingly',
					'zh-CN': '更改主题颜色时, 现在启动器的阴影颜色也会随之变动',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.2',
		version: '1.7.2',
		publishedAt: '2026-08-10',
		title: {
			'en-US': 'Axolotl Launcher 1.7.2',
			'zh-CN': 'Axolotl Launcher 1.7.2',
		},
		changes: {
			added: [
				{
					'en-US':
						'Added JVM argument presets, currently including Fallen’s Mojang authentication service HTTP forwarding.',
					'zh-CN': '添加了JVM参数预设功能, 目前内置Fallen的Mojang认证服务HTTP转发。',
				},
				{
					'en-US':
						'Added Mojang authentication as a resource mirror configuration to the settings interface, now set to automatic to automatically switch to Fallen’s authentication service when the Mojang authentication service is unavailable. Mitigations include but are not limited to errors such as "Authentication server down" when logging in with a valid account.',
					'zh-CN':
						'将Mojang认证作为资源镜像配置添加到设置界面, 现在设置为自动即可在Mojang认证服务不可用时自动切换到Fallen的认证服务。缓解包括但不限于正版登录时出现“认证服务器宕机”之类的报错。',
				},
				{
					'en-US':
						'Added a custom UUID configuration for offline login, along with a UUID copy button to directly copy the UUID.',
					'zh-CN': '离线登陆可以配置自定义UUID, 并且添加了UUID复制按钮, 可直接复制UUID。',
				},
				{
					'en-US':
						'Added a collapse button for ungrouped instances, allowing users to collapse and hide the list of ungrouped instances.',
					'zh-CN': '为未分组的实例添加了折叠按钮, 可以折叠隐藏未分组的实例列表。',
				},
				{
					'en-US':
						'Added automatic backup of instance settings to the instance folder, allowing users to restore the instance after a database loss.',
					'zh-CN': '数据库将自动备份实例的设置到实例文件夹, 以便在数据库丢失后恢复实例。',
				},
				{
					'en-US':
						'Added automatic backup of instance settings to the instance folder, allowing users to restore the instance after a database loss.',
					'zh-CN': '支持导入本就是本启动器的实例文件夹, 会完全保留实例设置。',
				},
				{
					'en-US':
						'Added a "Auto Import Missing Files" button in Settings. You can also customize the monitoring location. When enabled, the launcher will automatically scan files in the monitoring location and automatically import missing files into the instance when scenarios such as downloading an integration package fail.',
					'zh-CN':
						'在设置中新增了自动导入缺失文件开关按钮, 并可自定义监控位置。当启用时, 启动器会自动扫描监控位置的文件, 并将下载整合包等场景文件下载失败时, 自动把缺失的文件导入到实例中。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed the issue of Chinese encoding parsing errors in the Location header returned by mirror sites.',
					'zh-CN': '修复了镜像站返回Location 头中中文编码方式解析错误的问题。',
				},
				{
					'en-US':
						'Fixed the issue of database records for renamed files being lost during migration, now automatically merging and migrating records based on hash, and rebuilding ownership based on Modrinth hash when original ownership is lost.',
					'zh-CN':
						'现在会自动按照hash合并、迁移重命名文件的数据库记录, 在原归属丢失时依据 Modrinth hash 重建归属。',
				},
				{
					'en-US':
						'Fixed update checks for mods and other instance content using a permanent cache, so newly published updates could stay hidden even after refreshing; refreshing now rechecks the latest versions.',
					'zh-CN':
						'修复实例中模组等内容的更新检查使用永久缓存, 发布新版本后刷新仍不显示的问题；现在刷新会重新检查最新版本。',
				},
				{
					'en-US':
						'Fixed false "update available" badges when the installed file was already included in the target version or the installed version was identified incorrectly.',
					'zh-CN':
						'修复已安装文件已包含在目标版本中、或当前安装版本被识别错误时, 没有新版本却仍显示“可更新”的问题。',
				},
			],
			changed: [
				{
					'en-US': 'Removed the shadow around the recipe generator background edge',
					'zh-CN': '移除了配方生成器背景边缘的阴影',
				},
				{
					'en-US':
						'When downloading modpacks and other files, if a mod download fails, the launcher now provides three options to continue downloading the modpack: "Launcher re-download", "Manual import of missing files", and "Monitor folder for automatic import of files". This avoids rolling back the entire download due to a single file failure.',
					'zh-CN':
						'当下载整合包等文件时, 如果模组下载失败, 现在会提供 启动器重新下载/手动导入缺失文件/监控文件夹自动导入文件 三个选项来继续下载整合包, 避免一个文件下载失败直接全部回滚。',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.1',
		version: '1.7.1',
		publishedAt: '2026-08-08',
		title: {
			'en-US': 'Axolotl Launcher 1.7.1',
			'zh-CN': 'Axolotl Launcher 1.7.1',
		},
		changes: {
			added: [
				{
					'en-US': 'Added zh-cn locales for seed map biome picker',
					'zh-CN': '为种子地图中的群系选择器添加了中文本地化',
				},
				{
					'en-US': 'Added progress display for exporting modpacks.',
					'zh-CN': '为导出整合包添加了进度显示。',
				},
				{
					'en-US':
						'Added a recipe generator in Lab for creating custom crafting tables and other datapacks.',
					'zh-CN': '实验室新增配方生成器, 可自制合成表等数据包。',
				},
				{
					'en-US': 'Added a mod translation tool in Lab for translating mod content.',
					'zh-CN': '实验室新增模组翻译工具, 可翻译模组内容。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed the installer Browse buttons not opening a folder picker when the default destination did not exist yet.',
					'zh-CN': '修复默认目标目录尚未创建时, 安装程序的“浏览”按钮无法打开文件夹选择器的问题。',
				},
				{
					'en-US':
						'Fixed the installer remaining open after installation when Launch when complete was selected; it now closes after the launcher starts successfully and stays open if launching fails.',
					'zh-CN':
						'修复勾选“完成后启动”时安装程序不会自动退出的问题；启动器成功启动后安装程序会退出, 启动失败时则保留窗口。',
				},
				{
					'en-US':
						'Fixed the issue of the maximum page number being displayed incorrectly on the search page.',
					'zh-CN': '修复搜索页面最大页码显示错误的问题',
				},
				{
					'en-US': 'Fixed some issues on Linux.',
					'zh-CN': '修复了 Linux 下的一些问题。',
				},
			],
			changed: [
				{
					'en-US': 'Improved performance when exporting modpacks.',
					'zh-CN': '优化了导出整合包时的性能问题',
				},
				{
					'en-US':
						'Fixed the issue of search result translations being switched from the general translation API to mcim.',
					'zh-CN': '将搜索结果的翻译由通用翻译API切换至mcim',
				},
			],
		},
	},
	{
		id: 'launcher-1.7.0',
		version: '1.7.0',
		publishedAt: '2026-08-06',
		title: {
			'en-US': 'Axolotl Launcher 1.7.0',
			'zh-CN': 'Axolotl Launcher 1.7.0',
		},
		changes: {
			added: [
				{
					'en-US': 'Added AI integrations for translation and launcher assistance.',
					'zh-CN': '新增 AI 集成功能, 支持翻译和启动器辅助功能。',
				},
			],
			changed: [
				{
					'en-US': 'Improved translation logic and AI integration for more consistent results.',
					'zh-CN': '优化翻译逻辑和 AI 集成, 提升翻译结果的一致性。',
				},
				{
					'en-US':
						'Simplified download error dialogs to make failures easier to understand and recover from.',
					'zh-CN': '简化下载错误提示框, 让失败原因和恢复操作更清晰。',
				},
				{
					'en-US': 'Improved mod-related downloads for more reliable content installation.',
					'zh-CN': '优化模组相关下载, 提升内容安装的可靠性。',
				},
			],
			fixed: [
				{
					'en-US': 'Fixed downloads failing when the connection was slow but still active.',
					'zh-CN': '修复网络速度较低但仍在传输时下载失败的问题。',
				},
				{
					'en-US':
						'Fixed Minecraft account avatars sometimes remaining on the default skin after startup; failed skin loads now retry automatically and refresh after navigation.',
					'zh-CN':
						'修复 Minecraft 账号头像在启动后偶尔持续显示默认皮肤的问题；皮肤加载失败后现在会自动重试, 并在切换页面时重新获取。',
				},
				{
					'en-US':
						'Fixed an issue where the import instance window would flash all import options when closed or when clicking the "What can I drop?" button.',
					'zh-CN': '修复了导入实例窗口关闭或点击 我可以拖入什么 按钮时, 全部导入选项闪现的问题',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.12',
		version: '1.6.12',
		publishedAt: '2026-08-04',
		title: {
			'en-US': 'Axolotl Launcher 1.6.12',
			'zh-CN': 'Axolotl Launcher 1.6.12',
		},
		changes: {
			added: [
				{
					'en-US': 'Completely redesigned homepage with widgetized components',
					'zh-CN': '完全重新设计主页, 使其小组件化',
				},
			],
			changed: [
				{
					'en-US': 'Enhanced Windows icon rendering',
					'zh-CN': '优化软件在Windows下图标表现',
				},
			],
			fixed: [
				{
					'en-US':
						'CurseForge files bundled inside a modpack now remain in the modpack group, and existing instances are reconciled automatically without reclassifying user-added content.',
					'zh-CN':
						'CurseForge 整合包内置的文件现在会正确归入整合包分组；已有实例会自动校准, 且不会误归类用户后来添加的内容。',
				},
				{
					'en-US':
						'Modpack group rows now fall back to the instance icon when provider artwork is missing.',
					'zh-CN': '整合包平台图标缺失时, 内容分组现在会正确回落显示实例图标。',
				},
				{
					'en-US': 'Fixed local mods without a content record failing to enable or disable.',
					'zh-CN': '修复未建立内容记录的本地 Mod 无法正常启用或禁用的问题。',
				},
				{
					'en-US':
						'Fixed content toggles reverting visually after a mod was successfully enabled or disabled.',
					'zh-CN': '修复 Mod 成功启用或禁用后, 内容开关在界面上回弹的问题。',
				},
				{
					'en-US':
						'Fixed slow but active downloads being repeatedly canceled when they fell below the route-switching speed threshold; fallback attempts now continue until completion.',
					'zh-CN':
						'修复弱网下仍在传输的下载因低于线路切换速度阈值而被反复中止的问题；保底下载现在会持续到完成。',
				},
				{
					'en-US':
						'Fixed modpack missing-file warnings so affected files are named and can be restored directly; blank CurseForge mirror responses and stale states no longer leave files stuck as missing.',
					'zh-CN':
						'修复整合包文件缺失提示：现在会列出具体文件并可直接恢复；CurseForge 镜像空响应和陈旧状态不再导致文件持续显示为缺失。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.11',
		version: '1.6.11',
		publishedAt: '2026-08-04',
		title: {
			'en-US': 'Axolotl Launcher 1.6.11',
			'zh-CN': 'Axolotl Launcher 1.6.11',
		},
		changes: {
			added: [
				{
					'en-US':
						"The world editor can change a world's game mode, difficulty, cheats toggle and seed.",
					'zh-CN': '世界编辑器支持修改存档的游戏模式、难度、作弊开关与世界种子。',
				},
				{
					'en-US':
						'Game rules can now be edited with localized names, category grouping, search, and one-click reset to the vanilla default.',
					'zh-CN': '支持编辑游戏规则：规则名称已本地化, 按分类分组, 可搜索并一键恢复默认值。',
				},
				{
					'en-US':
						'The world editor backs up level.dat before saving and stays read-only while the world is open in game.',
					'zh-CN': '世界编辑器保存前会自动备份 level.dat, 存档正在游戏中打开时会自动进入只读状态。',
				},
				{
					'en-US':
						'Added automatic high-performance GPU selection for Minecraft on Linux, supporting AMD and NVIDIA systems.',
					'zh-CN': '新增 Linux 高性能显卡自动选择, 支持 AMD 和 NVIDIA 显卡运行 Minecraft。',
				},
			],
			changed: [
				{
					'en-US':
						'Editing a singleplayer world now opens a full editor page instead of a small dialog.',
					'zh-CN': '单人存档的“编辑”入口从小弹窗升级为完整的编辑页面。',
				},
				{
					'en-US':
						'Improved the Traditional Chinese (Taiwan) interface translation with hundreds of revised entries. Thanks to @DonkeyBear for the contribution.',
					'zh-CN': '改进繁体中文（台湾）界面翻译：修订数百条文案。感谢 @DonkeyBear 的贡献。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed an upgrade failure that could prevent the launcher from opening when existing modpack content contained duplicate records.',
					'zh-CN': '修复旧版整合包内容存在重复记录时升级失败, 导致启动器无法启动的问题。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.10',
		version: '1.6.10',
		publishedAt: '2026-08-03',
		title: {
			'en-US': 'Axolotl Launcher 1.6.10',
			'zh-CN': 'Axolotl Launcher 1.6.10',
		},
		changes: {
			added: [
				{
					'en-US':
						'Minecraft account avatars now render supported skin outer layers with a layered 2D effect and silhouette shadow.',
					'zh-CN': 'Minecraft 账号头像现支持渲染皮肤外层, 并以分层 2D 效果和轮廓阴影显示。',
				},
			],
			changed: [
				{
					'en-US':
						'Reworked instance content management so local files and modpack groups remain visible and manageable when an online provider is unavailable.',
					'zh-CN':
						'重构实例内容管理, 在线内容提供方不可用时, 本地文件和整合包分组仍会完整显示并可正常管理。',
				},
				{
					'en-US':
						'One-click content updates now update only content added after installation; modpack updates remain separate and preserve added content and local overrides.',
					'zh-CN':
						'一键更新现在仅更新安装整合包后添加的内容；整合包更新保持独立, 并会保留后装内容和本地覆盖。',
				},
				{
					'en-US':
						'Launcher networking now follows the operating system proxy automatically without a separate proxy toggle.',
					'zh-CN': '启动器网络现在会自动跟随操作系统代理, 无需单独配置代理开关。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed Minecraft account avatars sometimes failing to display or shifting when hovering account options.',
					'zh-CN': '修复 Minecraft 账号头像偶尔无法显示, 以及悬停账号选项时发生抖动的问题。',
				},
				{
					'en-US': 'Fixed an oversized border around the expanded account selector.',
					'zh-CN': '修复账号选择框展开时出现粗重边框的问题。',
				},
				{
					'en-US':
						'Fixed CurseForge author-restricted files opening invalid download pages, failing to import after browser download, or reporting completion before all files were present.',
					'zh-CN':
						'修复 CurseForge 作者限制文件打开错误下载页、浏览器下载后无法导入, 以及文件未齐时提前提示完成的问题。',
				},
				{
					'en-US':
						'Fixed incorrect content counts and missing-file warnings caused by shader configuration sidecar files being treated as shader packs.',
					'zh-CN': '修复光影配置附属文件被误识别为光影包, 导致内容数量和文件缺失提示错误的问题。',
				},
				{
					'en-US':
						'Fixed content refresh and manual import operations intermittently failing because the local database was locked.',
					'zh-CN': '修复内容刷新和手动导入偶发因本地数据库锁定而失败的问题。',
				},
				{
					'en-US':
						'Fixed incomplete faces on blocks next to observers, redstone dust, lanterns, hoppers, repeaters, extended pistons, and other non-full blocks in Schematic workshop.',
					'zh-CN':
						'修复了投影工坊中侦测器、红石粉、灯笼、漏斗、中继器、伸出的活塞及其他非完整方块导致相邻方块渲染不全的问题。',
				},
				{
					'en-US':
						'Fixed the camera occasionally changing direction abruptly during smooth mouse movement in read-only walk preview.',
					'zh-CN': '修复了只读漫游预览中平滑移动鼠标时视角方向偶尔突变的问题。',
				},
				{
					'en-US':
						'Fixed walk speed adjustment by scroll wheel conflicting with scrolling the materials list.',
					'zh-CN': '修复了只读漫游预览中滚轮调速与材料列表滚动冲突的问题。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.9',
		version: '1.6.9',
		publishedAt: '2026-08-02',
		title: {
			'en-US': 'Axolotl Launcher 1.6.9',
			'zh-CN': 'Axolotl Launcher 1.6.9',
		},
		changes: {
			added: [
				{
					'en-US': 'Launcher will now show a discord rich presence binded to Axolotl Launcher.',
					'zh-CN': '启动器现在会显示 Axolotl Launcher 的 Discord Rich Presence。',
				},
				{
					'en-US': 'Launcher will now show a discord rich presence with a more detailed status.',
					'zh-CN': '启动器现在会显示带有更详细状态的 Discord Rich Presence。',
				},
				{
					'en-US': 'Added download source priority controls and an optional system proxy setting.',
					'zh-CN': '新增下载源优先级选项与可选的系统代理设置。',
				},
			],
			changed: [
				{
					'en-US':
						'Improved download routing, concurrency, segmented transfers, and stalled-tail recovery for faster installs.',
					'zh-CN': '优化下载路由、并发、分段传输与慢尾恢复, 提升整体安装速度。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed inaccurate speed and ETA reporting and downloads appearing stuck at 99% or 100%.',
					'zh-CN': '修复下载速度与剩余时间显示不准, 以及进度卡在 99% 或 100% 的问题。',
				},
				{
					'en-US':
						'Fixed the issue of administrator judgment on Windows not matching actual needs.',
					'zh-CN': '修复了Windows下管理员判断与实际需求不符的问题。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.8',
		version: '1.6.8',
		publishedAt: '2026-08-02',
		title: {
			'en-US': 'Axolotl Launcher 1.6.8',
			'zh-CN': 'Axolotl Launcher 1.6.8',
		},
		changes: {
			added: [
				{
					'en-US':
						'Mods and resource packs that are not linked to an online project now show the icon packed inside the file.',
					'zh-CN': '未关联到线上项目的模组与资源包,现在会显示包内自带的图标。',
				},
				{
					'en-US':
						'Added a rollback button for content updates, allowing users to revert to the previous version after updating mods, resource packs, and other content.',
					'zh-CN':
						'新增内容更新后悔药,现在更新Mod、资源包等内容后,提供一个按钮可以回退到上一个版本。',
				},
				{
					'en-US':
						'Fixed schematics stored in nested folders not being recognized, now they are folded into a hierarchical view.',
					'zh-CN': '实例内容页面的投影项右边添加了编辑按钮,可直接导入投影工坊。',
				},
			],
			changed: [
				{
					'en-US':
						'Text in the launcher interface can no longer be selected by mouse by accident; editable fields are still selectable.',
					'zh-CN': '界面文本不再能被鼠标直接选中,避免误选；输入框等可编辑区域不受影响。',
				},
				{
					'en-US':
						'Optimized the caching of empty responses from online sources, which previously would be cached for 30 minutes and caused a poor experience; now empty responses are treated as unavailable, automatically falling back to available sources and updating immediately on next launch.',
					'zh-CN':
						'优化空返回也会被写入缓存,必须等待30min的不好体验,现在遇到空返回时判断为不可用,自动回退到可用源且下次启动立即更新。',
				},
				{
					'en-US':
						'Refactored and cleaned up legacy code paths for better reliability and easier maintenance.',
					'zh-CN': '重构并清理了部分历史遗留代码,提升稳定性与可维护性。',
				},
				{
					'en-US':
						'Improved nested-folder detection for modpacks and other resources, so files in deeper directories are recognized correctly.',
					'zh-CN': '增强了整合包等资源的嵌套识别,深层目录甚至是压缩包中的文件现在能被正确识别。',
				},
				{
					'en-US': 'Improved performance when enabling or disabling resources in bulk.',
					'zh-CN': '提升了批量修改资源启用状态时的性能。',
				},
			],
			fixed: [
				{
					'en-US':
						'Fixed the issue of the toggle switch bouncing back when disabling content, now the switch follows correctly.',
					'zh-CN': '修复了禁用内容时开关回弹的现象,现在开关正常跟手。',
				},
				{
					'en-US':
						'The custom system prompt for OpenAI-compatible translation services is now saved correctly and used for translations.',
					'zh-CN':
						'修复了 OpenAI 兼容翻译服务的自定义系统提示词无法保存的问题,现在会正确保存并在翻译时生效。',
				},
				{
					'en-US':
						'Fixed legacy Modrinth code so the right-click icon edit button now leads to the correct instance edit page instead of a blank page.',
					'zh-CN':
						'修复曾经modrinth遗留代码, 右键图标的编辑按钮现在通向正确的实例编辑界面而非空白页。',
				},
				{
					'en-US':
						'Fixed the conflict between global drag-and-drop import and the Schematic workshop; dragging and dropping schematic files in the Schematic workshop now imports them directly into the workshop instead of globally.',
					'zh-CN':
						'修复全局拖拽导入和投影工坊的打架问题,在投影工坊界面拖拽导入的投影文件现在会直接导入到投影工坊而不是全局导入。',
				},
				{
					'en-US':
						'Fixed the issue of download tasks not being cancellable, now they can be cancelled normally.',
					'zh-CN': '修复了下载任务无法取消的问题,现在可以正常取消下载任务。',
				},
				{
					'en-US': 'Fixed the issue of some files being locked in certain cases.',
					'zh-CN': '修复了部分情况下的文件自锁问题。',
				},
				{
					'en-US': 'Forge and NeoForge mods now show their name and icon correctly.',
					'zh-CN': '修复了 Forge/NeoForge 模组无法正常显示名称与图标的问题。',
				},
				{
					'en-US':
						'Fixed CurseForge projects with more than 50 files showing an incomplete version list; all published versions now appear.',
					'zh-CN':
						'修复了 CurseForge 项目文件数超过 50 时版本列表不翻页的问题,现在会显示全部已发布版本。',
				},
				{
					'en-US': 'Fixed a crash that could occur when uploading files.',
					'zh-CN': '修复了上传文件时可能崩溃的问题。',
				},
				{
					'en-US':
						'Fixed schematics stored in nested folders not being recognized, now they are folded into a hierarchical view.',
					'zh-CN': '修复了嵌套在子文件夹中的投影文件无法被识别的问题,现在会折叠分级显示文件层级。',
				},
				{
					'en-US': 'Fixed an issue where mods could not be disabled properly.',
					'zh-CN': '修复了模组无法被正常关闭的问题。',
				},
				{
					'en-US':
						'Fixed an OOM issue caused by a low-performance upload interface, which has now been removed.',
					'zh-CN': '修复了低性能的上传接口导致的OOM问题,现在直接移除了这个接口。',
				},
				{
					'en-US':
						'Fixed an issue where resources were not displayed correctly after adding them without an immediate refresh; a refresh button is now provided to manually refresh the resource list.',
					'zh-CN':
						'修复了添加资源后没有立即刷新导致的资源显示不正确的问题,现在提供一个刷新按钮来手动刷新资源列表。',
				},
				{
					'en-US': 'Resolved several known issues.',
					'zh-CN': '解决了一些已知问题。',
				},
			],
			security: [
				{
					'en-US': 'Added extra safeguards for unusual edge cases.',
					'zh-CN': '针对部分极端情况增加了安全处理,提升健壮性。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.7',
		version: '1.6.7',
		publishedAt: '2026-08-01',
		title: {
			'en-US': 'Axolotl Launcher 1.6.7',
			'zh-CN': 'Axolotl Launcher 1.6.7',
		},
		changes: {
			fixed: [
				{
					'en-US':
						'Fixed schematics saved with reversed selection axes appearing upside down or mirrored in Schematic workshop.',
					'zh-CN': '修复了使用反向选区轴保存的投影在投影工坊中上下颠倒或镜像的问题。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.6',
		version: '1.6.6',
		publishedAt: '2026-08-01',
		title: {
			'en-US': 'Axolotl Launcher 1.6.6',
			'zh-CN': 'Axolotl Launcher 1.6.6',
		},
		changes: {
			added: [
				{
					'en-US':
						'Imported instances now automatically recognize and set icons based on their mod loader.',
					'zh-CN': '导入的实例现在会根据加载器自动识别并设置图标。',
				},
				{
					'en-US':
						'Added Schematic workshop in Lab. Open local or instance .litematic and .schem files to inspect builds in 3D, measure and manage layers and materials, edit blocks, and export your work.',
					'zh-CN':
						'实验室新增投影工坊：可打开本地或实例内的 .litematic 和 .schem 文件, 在 3D 工作区查看建筑、测量并管理层级和材料、编辑方块, 以及导出修改后的投影。',
				},
			],
			changed: [
				{
					'en-US':
						'Adjusted the position of source filter buttons on the Discover page for better usability.',
					'zh-CN': '调整了发现页的来源筛选按钮位置, 提升使用体验。',
				},
				{
					'en-US':
						'Manual CurseForge downloads skipped during an installation now remain listed after the task finishes, making them easier to complete later.',
					'zh-CN':
						'安装过程中跳过的 CurseForge 手动下载现在会在任务完成后保留在列表中, 便于稍后完成。',
				},
			],
			fixed: [
				{
					'en-US': 'Fixed breadcrumbs not matching the actual page navigation.',
					'zh-CN': '修复了面包屑与实际页面不一致的问题。',
				},
				{
					'en-US':
						'Fixed Modrinth update checks so CurseForge-tracked files are not suggested as Modrinth updates, while eligible manually added content can still be matched.',
					'zh-CN':
						'修复 Modrinth 更新检查：由 CurseForge 跟踪的文件不再被当作 Modrinth 更新推荐, 同时符合条件的手动添加内容仍可匹配更新。',
				},
				{
					'en-US':
						'Fixed the issue of external import of modpacks not being able to update mods with one click',
					'zh-CN': '修复了外部导入整合包无法一键更新mod的问题。',
				},
				{
					'en-US': 'Fixed the issue of CF limiting resource downloads in some cases',
					'zh-CN': '修复了部分情况下CF限制资源下载提示消失问题。',
				},
				{
					'en-US': 'Fixed some Chinese copywriting issues.',
					'zh-CN': '修复了部分中文文案。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.5',
		version: '1.6.5',
		publishedAt: '2026-07-31',
		title: {
			'en-US': 'Axolotl Launcher 1.6.5',
			'zh-CN': 'Axolotl Launcher 1.6.5',
		},
		changes: {
			fixed: [
				{
					'en-US': 'Fixed the issue of disappearing online content',
					'zh-CN': '修复了联机消失问题',
				},
				{
					'en-US': 'Fixed the issue of some code being rolled back',
					'zh-CN': '修复了部分代码被回滚的情况',
				},
				{
					'en-US': 'Fixed some known issues',
					'zh-CN': '解决了一些已知问题',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.4',
		version: '1.6.4',
		publishedAt: '2026-07-31',
		title: {
			'en-US': 'Axolotl Launcher 1.6.4',
			'zh-CN': 'Axolotl Launcher 1.6.4',
		},
		changes: {
			changed: [
				{
					'en-US': 'Improved download speed for more efficient content installation.',
					'zh-CN': '优化下载速度, 内容安装更加高效。',
				},
				{
					'en-US':
						'Disabled automatic updates in portable mode. Portable users should update manually from GitHub.',
					'zh-CN': '便携模式下禁用自动更新, 便携版用户请前往 GitHub 手动更新。',
				},
				{
					'en-US':
						'Removed automatic redirect to the Create page when no instances exist. Users can now view the empty home page.',
					'zh-CN': '移除了无实例时自动跳转到创建页面的行为, 现在可以正常浏览空白首页。',
				},
				{
					'en-US': 'Optimized instance page caching to avoid reloading data on every visit.',
					'zh-CN': '优化实例页面缓存机制, 避免每次访问时重新加载数据。',
				},
				{
					'en-US':
						'Enhanced the instance content page refresh button to re-fetch mod online information.',
					'zh-CN': '实例内容页面的刷新按钮现在可以重新获取模组的在线信息。',
				},
			],
			added: [
				{
					'en-US': 'Added a back-to-top button on the instance content page for easier navigation.',
					'zh-CN': '实例内容页面新增回到顶部按钮, 长页面浏览更加便捷。',
				},
			],
		},
	},
	{
		id: 'launcher-1.6.1',
		version: '1.6.1',
		publishedAt: '2026-07-29',
		title: {
			'en-US': 'Axolotl Launcher 1.6.1',
			'zh-CN': 'Axolotl Launcher 1.6.1',
		},
		changes: {
			changed: [
				{
					'en-US':
						'Redesigned Java management with clearer default-version controls and a more streamlined download experience.',
					'zh-CN': '优化 Java 管理界面与交互, 更清晰地管理各版本默认 Java, 并简化下载流程。',
				},
				{
					'en-US': 'Improved the Downloads page layout and actions for easier task management.',
					'zh-CN': '优化下载页面的布局与操作, 下载任务管理更加便捷。',
				},
			],
			fixed: [
				{
					'en-US': 'Fixed the game version selector being obscured on the Seed Map page.',
					'zh-CN': '修复种子地图中的游戏版本选择器被意外遮挡的问题。',
				},
				{
					'en-US': 'Fixed Minecraft being incorrectly reported as crashed after a normal exit.',
					'zh-CN': '修复正常退出游戏后被错误报告为崩溃的问题。',
				},
				{
					'en-US': 'Fixed missing dependencies in macOS builds.',
					'zh-CN': '修复 macOS 构建缺少依赖的问题。',
				},
			],
		},
	},
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
					'zh-CN': '新增实验室, 首批提供渐变颜色生成器和 Java 版种子地图。',
				},
			],
			changed: [
				{
					'en-US':
						'Improved download routing, retries, and progress reporting for more reliable installs.',
					'zh-CN': '优化下载源切换、重试与进度展示, 提升安装下载的稳定性。',
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
					'zh-CN': '为避免批量导入过程中还未来得及解析的加载器造成误解, 现在逐个导入实例并显示进度',
				},
				{
					'en-US':
						'Improved the Linux desktop file (.desktop) with Comment, Keywords, StartupWMClass, and StartupNotify fields; added x-scheme-handler/axolotl protocol association and Chinese localization; and set WEBKIT_DISABLE_DMABUF_RENDERER=1 for Exec.',
					'zh-CN':
						'优化 Linux 桌面文件（.desktop）：补充 Comment、Keywords、StartupWMClass、StartupNotify 等字段, 添加 x-scheme-handler/axolotl 协议关联与中文本地化, 并为 Exec 添加 WEBKIT_DISABLE_DMABUF_RENDERER=1 环境变量。',
				},
				{
					'en-US':
						'Replaced Tauri template variables in the Linux desktop file template with fixed values, ensuring the built .desktop file uses "Axolotl Launcher" directly for its name, icon, and executable.',
					'zh-CN':
						'将 Linux 桌面文件模板从 Tauri 模板变量格式改为固定值格式, 确保编译后的 .desktop 文件直接使用 "Axolotl Launcher" 作为名称、图标和可执行文件。',
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
					'zh-CN': '离线模式提示中新增刷新按钮, 无需重启启动器即可重新检测会话服务器连接状态。',
				},
				{
					'en-US':
						'Interrupted downloads of large files now resume from where they left off instead of restarting from zero, including after switching download sources or retrying a failed install.',
					'zh-CN':
						'大文件下载中断后现在会从断点继续, 而不是从头重新下载——切换下载源或重试失败的安装时同样生效。',
				},
				{
					'en-US':
						'Project pages now link to the matching MC Mod (mcmod.cn) wiki page — in the sidebar links and the top-right menu — when the project is found in the bundled wiki index. Works for both Modrinth and CurseForge projects.',
					'zh-CN':
						'项目详情页现在会链接到对应的 MC 百科（mcmod.cn）页面——位于侧栏相关链接和右上角菜单中, 仅当项目能在内置百科索引中找到时显示。Modrinth 和 CurseForge 项目均支持。',
				},
			],
			changed: [
				{
					'en-US':
						"Checking a modpack's contents no longer loads the entire pack file into memory; it now streams to the download cache and is reused by a later install of the same version.",
					'zh-CN':
						'解析整合包内容时不再将整个整合包文件载入内存, 而是流式下载到缓存, 之后安装同一版本时可直接复用。',
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
						'修复了关闭导入方式弹窗时因无限循环导致卡死的问题, 同时「取消」现在是真正的按钮。',
				},
				{
					'en-US':
						'Forge, Fabric, and NeoForge files can now fall back to their official servers when download mirrors are unavailable or have not synced a newly released version yet.',
					'zh-CN':
						'当下载镜像不可用或尚未同步新发布的版本时, Forge、Fabric 和 NeoForge 文件现在会回退到官方服务器下载。',
				},
				{
					'en-US':
						'Servers that mishandle multi-connection downloads are now remembered during a session, so large files stop wasting a doomed segmented attempt before every download.',
					'zh-CN':
						'不支持多线程分段下载的服务器现在会在会话内被记住, 大文件不再每次下载都先经历一轮注定失败的分段尝试。',
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
						'导入实例不再在导入真正完成前提示成功——导入失败时现在会提示错误, 而不是错误地提示成功。',
				},
				{
					'en-US':
						'Changing the app directory now moves shared instance links without moving or copying their original files.',
					'zh-CN': '更改应用目录时, 现在仅迁移共享实例链接, 不再移动或复制其原始文件。',
				},
				{
					'en-US':
						'Creating a custom instance once again defaults its icon to the selected mod loader (Fabric, Forge, Quilt, NeoForge) instead of the generic placeholder.',
					'zh-CN':
						'创建自定义实例时, 图标重新默认使用所选加载器的图标（Fabric、Forge、Quilt、NeoForge）, 不再是通用占位图。',
				},
				{
					'en-US':
						'Loader and other newer built-in instance icons now display without the avatar frame, matching the rest of the built-in icons.',
					'zh-CN': '加载器及其他较新的内置实例图标现在与其余内置图标一致, 不再带边框显示。',
				},
				{
					'en-US':
						'Fixed the launcher failing to start with a "Cannot save an incomplete Java installation" error when a leftover unfinished Java download was found while changing the app directory or migrating old launcher data.',
					'zh-CN':
						'修复更改应用目录或迁移旧启动器数据时, 遗留的未完成 Java 下载会导致启动器无法启动并报 "Cannot save an incomplete Java installation" 错误的问题。',
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
					'zh-CN': '设置 > 外观新增「透明背景」选项, 可通过滑块调节桌面透过启动器窗口显示的程度。',
				},
				{
					'en-US':
						'Added a background blur toggle for the transparent background, frosting whatever shows through the window.',
					'zh-CN': '透明背景新增「背景模糊」开关, 可将透出的画面做磨砂玻璃处理。',
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
					'zh-CN': '新增条目翻译功能, 让翻译功能可以应用到条目外的标题和介绍。',
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
					'zh-CN': '修改扫描逻辑, 优化导入扫描的部分石山, 提升兼容性。',
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
						'新增全局拖拽功能：直接拖入模组、资源包、光影包、存档、投影文件及启动器, 即可快速导入, 无需在菜单中翻找。',
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
						'新增模组导入校验：安装模组时, 启动器会自动检测其与当前 Minecraft 版本和加载器的兼容性, 不匹配时会提前提醒。',
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
						'中文界面下, 实例内容页与整合包内容弹窗的已装模组现以「中文名 (英文名)」显示, 并支持用中文搜索已装内容。',
				},
				{
					'en-US':
						'Under the Simplified Chinese locale, newly downloaded mods, resource packs, shader packs and data packs are saved as "[中文名]original-name" when a Chinese name is known; unknown files keep their original names and exported modpacks always restore the original file names.',
					'zh-CN':
						'中文界面下, 新下载的模组、资源包、光影包和数据包会以「[中文名]原文件名」保存；查不到中文名时保持原样, 导出整合包时自动还原为原文件名。',
				},
				{
					'en-US':
						'Browsing the Discover Content page without searching now also shows bilingual "中文名 (English)" titles under the Simplified Chinese locale, for both Modrinth and CurseForge results.',
					'zh-CN':
						'中文界面下, 「发现内容」页直接浏览（不搜索）时也会显示「中文名 (英文名)」双语标题, Modrinth 与 CurseForge 结果均生效。',
				},
				{
					'en-US':
						'The game language now follows the launcher language on the first launch of an instance, including imported modpacks, using the correct language code for each game version; instances you already play keep your in-game choice.',
					'zh-CN':
						'游戏语言现在会在实例首次启动时自动跟随启动器语言（包括导入的整合包）, 并按游戏版本写入正确的语言代码；已游玩过的实例仍保留游戏内的语言设置。',
				},
				{
					'en-US':
						'The left sidebar now animates the active highlight sliding between pages when switching sections, matching the content type tabs.',
					'zh-CN': '左侧导航栏切换页面时, 选中高亮改为滑动过渡动画, 与顶部内容类型标签栏保持一致。',
				},
				{
					'en-US':
						'You can now write a custom system prompt for OpenAI-compatible translation services (Settings > Translation).',
					'zh-CN': '现在可以在翻译设置中为 OpenAI 兼容服务编写自定义系统提示词。',
				},
				{
					'en-US':
						'Translation results now appear in staggered batches with a smooth floating animation.',
					'zh-CN': '翻译结果现在以逐批浮动动画显示, 视觉体验更流畅。',
				},
				{
					'en-US':
						'Added a Windows option to use the high-performance GPU for the launcher and Java.',
					'zh-CN': '新增 Windows 高性能显卡选项, 可用于启动器和 Java。',
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
						'优化整合包导入兼容性：支持更多整合包格式, 能更好地处理各种特殊情况, 导入成功率更高。',
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
						'加快 Java 检测：现在优先读取每个安装目录的元数据文件判断版本, 避免为每个候选启动 JVM, 减少首次扫描的耗时。',
				},
				{
					'en-US':
						'Downloading or launching an instance now scans the system for an already-installed Java of the required version before downloading a new runtime, reusing an existing installation instead of downloading a duplicate.',
					'zh-CN':
						'下载或启动实例时, 现在会先扫描本机是否已安装所需版本的 Java, 找到则复用, 仅在确实没有时才下载新的运行时, 避免重复下载。',
				},
				{
					'en-US':
						'Crash diagnostics now combine related logs and provide direct analysis and export actions.',
					'zh-CN': '崩溃诊断现在会归集相关日志, 并提供直接分析和导出操作。',
				},
				{
					'en-US':
						'The log console and local crash diagnosis are now fully localized in English, Simplified Chinese, and Traditional Chinese.',
					'zh-CN': '日志控制台与本地崩溃诊断现已完整支持英语、简体中文和繁体中文。',
				},
				{
					'en-US':
						'Empty log consoles now show Chinese startup guidance with a pink side-view axolotl illustration matching the launcher icon.',
					'zh-CN': '空日志控制台现在会显示中文启动提示, 以及贴近启动器图标的粉色美西螈侧视字符画。',
				},
				{
					'en-US':
						'Translation requests are now sent in batches (5 segments per batch) to reduce API overhead.',
					'zh-CN': '翻译请求现在分批发送（每批5个段落）, 降低 API 调用频率。',
				},
				{
					'en-US':
						'Offline account creation now warns when a Chinese username may be incompatible with Minecraft 1.18 and newer.',
					'zh-CN':
						'创建离线账户时, 若使用中文用户名, 现在会提示其可能与 Minecraft 1.18 及以上版本不兼容。',
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
						'修复数据库备份被写入 Modrinth 目录的问题, 现在改为保存到启动器自己的应用数据目录。',
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
						'扩展 Java 自动检测范围, 现可搜索 JAVA_HOME 同级安装、常见发行版目录、Minecraft 官方启动器运行时及可能的安装目录。',
				},
				{
					'en-US':
						'Added automatic memory allocation that adapts to available RAM and installed mods each time an instance launches.',
					'zh-CN': '新增自动分配内存, 可在每次启动实例时根据可用内存和已安装模组动态调整。',
				},
				{
					'en-US':
						'Added a live memory allocation display and one-click memory optimization on Windows.',
					'zh-CN': '新增实时内存分配展示, 并在 Windows 上提供一键内存优化。',
				},
			],
			changed: [
				{
					'en-US':
						'Java detection now caches results, scans sources concurrently, and refreshes the installation list in the background.',
					'zh-CN': 'Java 检测现在会缓存结果、并行扫描不同来源, 并在后台刷新安装列表。',
				},
				{
					'en-US':
						'The launcher now reuses an already detected Java runtime with the required version before downloading a new one.',
					'zh-CN':
						'启动实例缺少所需 Java 版本时, 现在会优先复用已检测到的同版本运行时, 再考虑下载新的运行时。',
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
					'zh-CN': '新增 HMCL、PCL2、PCL2CE 启动器实例导入支持, 可直接根据启动器解析出所有实例。',
				},
				{
					'en-US':
						'Added generic folder import — any directory containing a .minecraft folder can now be imported as an instance.',
					'zh-CN': '新增通用文件夹导入功能, 可导入任意含 .minecraft 的目录。',
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
						'新增拖放导入功能：直接拖入模组、资源包、光影包、存档、投影文件及启动器实例, 即可快速导入。',
				},
			],
			changed: [
				{
					'en-US':
						'Optimised copy_dotminecraft_with_reporter: serial copies are now concurrent, reducing time complexity from O(n·t) to O(max(t)), and progress reporting has been improved.',
					'zh-CN':
						'优化 copy_dotminecraft_with_reporter：串行复制改为并发, 时间复杂度由 O(n·t) 降为 O(max(t)), 优化进度上报时机。',
				},
				{
					'en-US': 'Updated shared instance indicators and warning hints for clarity.',
					'zh-CN': '更新共享实例标识与警告提示。',
				},
				{
					'en-US':
						'Greatly improved modpack import compatibility — now handles CurseForge, MCBBS, HMCL, MultiMC, PCL launcher-bundled archives and various non-standard pack formats.',
					'zh-CN':
						'大大增强整合包导入兼容性, 兼容 CurseForge、MCBBS、HMCL、MultiMC、PCL 等导出的附带启动器的整合包以及各种不完全符合规范的整合包格式。',
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
						'整合包导入现在按压缩包内容识别格式：除 .mrpack 外, 还支持 CurseForge、MCBBS、HMCL、MultiMC/Prism 导出包、附带启动器的整合包以及打包的游戏目录。',
				},
				{
					'en-US':
						'Added OptiFine support: modpacks declaring OptiFine install it automatically, standalone as the loader or as a mod alongside Forge/NeoForge.',
					'zh-CN':
						'新增 OptiFine 支持：声明了 OptiFine 的整合包会自动安装——单独存在时作为加载器, 与 Forge/NeoForge 共存时作为模组安装。',
				},
				{
					'en-US':
						'Added an appearance setting to limit the number of recent instances shown in the sidebar, with 0 showing all instances.',
					'zh-CN': '新增外观设置, 可限制侧边栏显示的最近实例数量, 设为 0 时显示全部实例。',
				},
				{
					'en-US':
						'Added custom accent colors with a preset palette, hue slider, hex input, and automatic light and dark theme variants.',
					'zh-CN':
						'新增自定义强调色, 支持预设色板、色相滑块、十六进制色号及自动生成浅色和深色主题变体。',
				},
			],
			changed: [
				{
					'en-US':
						'Improved the update settings version history with clearer release cards and details.',
					'zh-CN': '优化更新设置中的版本历史, 提供更清晰的发布卡片和详情展示。',
				},
				{
					'en-US':
						'The sidebar instance list now scrolls independently when it exceeds the available space.',
					'zh-CN': '侧边栏实例列表超出可用空间时, 现在可以独立滚动。',
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
						'修复本地整合包安装在 100% 后看似卡住, 以及 Minecraft 文件下载停止接收数据时任务无法结束的问题。',
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
					'zh-CN': '新增应用更新后的分类公告弹窗, 以及设置中的永久版本历史记录。',
				},
				{
					'en-US': 'Added a first-run onboarding guide that can also be replayed from settings.',
					'zh-CN': '新增首次使用引导, 并支持从设置中重新播放。',
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
					'zh-CN': '错误日志导出现在使用流式压缩, 降低大日志导出时的内存占用。',
				},
				{
					'en-US':
						'WARN and ERROR logs now rotate before the 30 MiB boundary without splitting individual events.',
					'zh-CN': 'WARN 和 ERROR 日志现在会在 30 MiB 边界内保持完整, 轮转时不会拆分单个事件。',
				},
				{
					'en-US': 'Launcher logs older than three days are now removed automatically.',
					'zh-CN': '启动器日志创建超过三天后现在会自动删除。',
				},
			],
			fixed: [
				{
					'en-US': 'Fixed skipped mods remaining in the list after manually installing them.',
					'zh-CN': '修复手动安装跳过下载的模组后, 已跳过模组列表不会更新的问题。',
				},
				{
					'en-US':
						'Fixed duplicate download events causing complete installation states to be logged repeatedly.',
					'zh-CN': '修复下载事件重复记录完整安装状态, 导致启动器日志快速膨胀的问题。',
				},
				{
					'en-US':
						'Fixed the Fabric/Modrinth content page watcher repeatedly writing the same map and getting stuck loading.',
					'zh-CN':
						'修复 Fabric/Modrinth 实例内容页 watcher 重复写入相同 Map, 触发递归更新并持续加载的问题。',
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
