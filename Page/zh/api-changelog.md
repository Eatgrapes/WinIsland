# 插件 API 更新日志

记录 `winisland-plugin-api` 的所有重要变更。

格式遵循 [Keep a Changelog](https://keepachangelog.com/),
版本管理遵循 [Semantic Versioning](https://semver.org/)。

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
