# 插件 API 更新日志

本页只记录已经发布的 `winisland-plugin-api` 版本。版本发布时直接添加对应记录，不再保留“未发布”区段。

## 0.3.0 - 2026-08-09

新增：

- 通过 `winisland_plugin_entry_v1() -> *const PluginDescriptorV1` 提供原生 DLL ABI v1
- 由宿主签发 `PluginToken` 和 `ResourceId`，用于身份与资源所有权校验
- Context、Media、国际化和 Host State 能力声明
- 通过 `HostApiV1::query_interface` 查询版本化服务
- Context 创建、更新、释放、优先级、紧凑文本和超时功能
- 支持封面、播放进度、可用控制和可选回调的 Media 资源
- 可释放的翻译 bundle，以及当前媒体和主题的 Host State 快照
- 使用 `id`、`abi-version` 和 `entry` 字段的单入口插件包

变更：

- **破坏性变更**：移除 0.2 的 `PluginVTable`、`PluginType`、`PluginInstanceC`、
  `HostApiC`、`plugin_get_instance` 和 `plugin_set_host_api` 接口
- **破坏性变更**：移除尚未形成闭环的 Theme 和 Shortcut 接口
- 生命周期严格调整为 `create -> shutdown -> destroy`；只有 shutdown 成功才卸载 DLL
- Context 标识改为宿主签发的数字资源，不再使用插件提供的字符串
- 插件 Media 不受 SMTC 开关影响，界面只显示插件声明的控制按钮
- 插件工作线程修改资源时会唤醒 WinIsland 事件循环
- Packager 现在读取 Cargo 的 `repository` 和 `[lib].name` 元数据

修复：

- Descriptor 大小、ABI 版本、能力、元数据和生命周期回调校验
- 与 token 绑定的资源所有权、每插件数量限制和内存限制
- Media 回调重入和插件卸载同步
- UTF-8 安全的定长缓冲区截断，以及有上限的借用切片复制
- Context 更新/释放事件合并和 Media seek 来源绑定
- 翻译 bundle 清理和宿主事件循环唤醒合并
- 有上限的 ZIP 解压、Windows 路径碰撞检查、staging 激活、备份和回滚

## 0.2.0 - 2026-06-19

新增：

- `TranslationPairC` — FFI 安全的插件国际化键值对
- `HostApiC::register_translations` — 插件可在 `on_load` 期间注册翻译；
  相同键的后续注册会覆盖先前值
- 国际化覆盖层 — `tr()` 会先检查插件注册的翻译，再检查 `.lang` 文件

变更：

- **破坏性变更**：`HostApiC` 新增必需字段 `register_translations`；
  所有宿主实现都必须提供该回调
- crate 拆分为职责明确的 host、vtable 和类型模块
- 所有公共类型仍从 crate 根目录重新导出

## 0.1.3 - 2026-06-19

新增：

- `MediaSourceC` — 插件可注入的媒体源（标题、艺人、专辑、时长、进度、封面）
- `HostApiC::set_media_source` — 用插件提供的媒体数据替代 SMTC
- `HostApiC::clear_media_source` — 恢复 SMTC 作为活动媒体源

变更：

- `HostApiC` 派生 `Clone`, `Copy` 以安全用于 FFI
- `PluginResultC` 派生 `Debug`, `Clone`, `Copy`
- `ContextDataC`, `ContextIdC`, `HostStateC` — 新增基于推送的上下文类型
- `PluginVTable::set_host_api` — 可选插槽，用于插件接收 `HostApiC` 指针

## 0.1.2 - 2026-06-17

新增：

- README.md 文档，包含 crate 级文档、使用示例和 feature flags

## 0.1.1 - 2026-06-16

新增：

- `packager` feature：`PluginPackager` 用于构建、签名和打包插件为 ZIP
- Cargo.toml crates.io 发布元数据（仓库地址、主页、许可证、关键词、分类）
- 启用 `packager` feature 的 `docs.rs` 配置

变更：

- 使用 `str_to_fixed` 辅助函数初始化字节缓冲区，替代手动填充循环
- Packager 在 `build()` 时验证 `manifest.yaml`；检查缺失字段和缓冲区大小
- `github_link` 字段现在为必填项（不可为空），以满足宿主验证

修复：

- `plugin_get_instance` 文档示例使用正确的 `#[no_mangle]` 导出，移除了多余的 `fn main`
- Packager 模块文档中的失效文档链接
- 签名流程中的 `BG_CACHE` 大小检查

## 0.1.0 - 2026-06-15

新增：

- 初始发布 — 将 C ABI 类型从 WinIsland 宿主提取到独立 crate
- 核心类型：`PluginInstanceC`, `PluginVTable`, `PluginMetadataC`, `IslandContentC`, `ThemeColorsC`, `AnimationConfigC`, `ShortcutC`, `PluginResultC`
- `PluginType` 枚举，支持 `from_u32` 转换
- `PluginGetInstanceFn` — 插件 DLL 的入口函数签名
- `str_to_fixed` / `read_c_str` / `read_opt_c_str` FFI 字节缓冲区处理辅助函数
- 优先级常量：`PRIORITY_LOW`, `PRIORITY_MEDIUM`, `PRIORITY_HIGH`
- 内容标签常量：`ISLAND_CONTENT_TAG_MUSIC`, `ISLAND_CONTENT_TAG_NOTIFICATION`, `ISLAND_CONTENT_TAG_STATUS`
