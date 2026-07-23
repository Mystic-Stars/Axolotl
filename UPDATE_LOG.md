# Release notes v1.5.0

- 新增 HMCL、PCL2、PCL2CE 启动器实例导入支持，可直接根据启动器解析出所有实例
- 新增通用文件夹导入功能，可导入任意含 .minecraft 的目录
- 新增添加为共享实例功能：导入时可选软链接而非复制
- 优化 copy_dotminecraft_with_reporter：串行复制改为并发，时间复杂度由 O(n·t) 降为 O(max(t))，优化进度上报时机
- 更新共享实例标识与警告提示
- 补齐文件标签页删除时的确认弹窗
- 大大增强整合包导入兼容性，现在已经可以兼容CurseForge、MCBBS 、HMCL 、MultiMC 、PCL等导出的附带启动器的整合包以及各种不完全符合规范整合包格式
- 新增 OptiFine 支持：整合包声明 OptiFine 时自动安装——单独存在时作为加载器
