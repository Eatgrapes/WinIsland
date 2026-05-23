# 上游合并影响说明

## 合并概要

- **上游仓库**：Eatgrapes/WinIsland (commit c6de735)
- **合并日期**：2026-05-24
- **合并状态**：未提交（等待验证）
- **合并策略**：冲突时优先保留上游代码，用户功能已部分重新应用到上游代码基础上

---

## 上游新增功能（已合入）

| 功能 | 涉及文件 |
|------|----------|
| 插件系统（ZIP 安装、API 类型定义）| `src/plugin/`, `crates/winisland-plugin-api/`, `Cargo.toml` |
| Dock 位置（支持顶部/底部/左/右六个位置）| `src/core/config.rs`, `src/window/app.rs`, `src/window/settings.rs`, `src/core/render.rs` |
| 触摸屏支持 | `src/window/app.rs` |
| 设置界面性能优化（缓存、帧率、滚动）| `src/window/settings.rs`, `src/utils/settings_ui/renderer.rs` |
| 进度条拖拽修复 | `src/window/app.rs`, `src/core/smtc.rs` |
| 音频多采样格式支持（F32/I16/U16）| `src/core/audio.rs` |
| `main_view.rs` 重命名为 `music_view.rs` | `src/ui/expanded/` |
| GitHub Issue/PR 模板 | `.github/` |
| 贡献规范文档 | `CONTRIBUTING.md`, `AGENTS.md`, `Docs/` |
| Clippy/fmt 修复 | 多个文件 |

---

## 用户功能状态

### ✅ 已成功保留的用户功能

| 功能 | 状态 |
|------|------|
| **config.rs**：settings_theme、mini_cover_shape、expanded_cover_shape、cover_rotate、audio_gate 配置项 | ✅ 已保留 |
| **audio.rs**：gate_override、set_gate_override()、SetMute(true) 静音、effective_gate | ✅ 已适应上游重构重新应用 |
| **smtc.rs**：diff_from_last 微小漂移过滤、切歌 800ms 延迟、封面属性一致性验证、自适应重试间隔 | ✅ 已重新应用 |
| **main.rs**：设置窗口单实例检测（CreateMutexW） | ✅ 已重新应用 |
| **Cargo.toml**：Win32_Graphics_Dwm feature（Mica 支持） | ✅ 已保留 |
| **mod.rs**：pub mod backdrop | ✅ 已保留 |
| **font.rs**：draw_text_with_custom_font()、draw_text_with_default_font() | ✅ 已重新应用 |
| **color.rs**：SettingsTheme、dark/light 主题 | ✅ 自动合并 |
| **backdrop.rs**：try_enable_mica()、get_dynamic_bg_color()、clear_dynamic_bg_cache() | ✅ 文件存在 |
| **lang 文件**：所有用户翻译键 | ✅ 自动合并 |
| **items.rs**：FontPreview 类型 | ✅ 自动合并 |
| **anim.rs**：is_animating() 方法 | ✅ 自动合并 |
| **settings_ui/anim.rs**：SwitchAnimator | ✅ 自动合并 |
| **PROJECT_UNDERSTANDING.md** | ✅ 文件存在 |

### ❌ 需要手动重新应用的用户功能

以下文件已恢复为上游版本，用户功能需要手动重新应用：

#### 1. `src/window/app.rs`（改动最大）

需要重新应用的功能：
- **Mica 支持**：窗口创建时调用 `try_enable_mica(hwnd)`
- **迷你态暂停控制按钮**：`get_mini_control_rects()` 导入和点击检测逻辑
- **音频门控联动**：`audio_gate` 配置与 `set_gate_override()` 调用
- **music_active 逻辑**：改为 `!media.title.is_empty()`（不再用 5 秒超时）
- **自动隐藏行为修复**：
  - `is_paused_idle` 逻辑
  - `is_idle` 包含 `is_paused_idle`
  - 自动隐藏恢复条件改为 `media.is_playing`
- **dt 计算**：`draw_island` 新增 dt 参数
- **配置检测优化**：
  - 间隔从 60 帧降为 30 帧
  - 检测 `island_style` 变化 → `clear_dynamic_bg_cache()`
  - 检测封面形状变化 → `clear_cover_cache()`
- **封面翻转**：歌曲切换时 `trigger_cover_flip()` + `clear_dynamic_bg_cache()`
- **暂停状态重绘**：只在播放时持续重绘（节省 CPU）
- **暂停状态歌词冻结**：`!is_paused` 条件
- **暂停状态歌词滚动停止**：`&& !is_paused` 条件

**注意**：上游重构了 `draw_island` 调用方式（使用 `DrawIslandParams` 命名参数结构体），用户功能需要适应这个新接口。

#### 2. `src/core/render.rs`

需要重新应用的功能：
- **封面形状支持**：`mini_cover_shape`、`expanded_cover_shape` 参数
- **封面旋转动画**：`MINI_COVER_ROTATION`、`cover_rotate` 参数
- **动态配色背景**：`get_dynamic_bg_color()`、`get_last_valid_color()` 集成
- **迷你态暂停控制**：`MINI_PAUSE_ANIM`、`get_mini_control_rects()` 函数
- **迷你态封面居中裁剪**：非正方形封面源矩形计算
- **毛玻璃/Mica/动态风格切换**：`island_style` 多风格处理
- **dt 参数**：传递给 `draw_widget_page`
- **widget_animating 返回值**：持续重绘支持

**注意**：上游重构为 `DrawIslandParams` 结构体传参方式，函数签名变了，需要适配。

#### 3. `src/ui/expanded/music_view.rs`（原 main_view.rs）

**注意**：上游已将 `main_view.rs` 重命名为 `music_view.rs`，函数名从 `draw_main_page` 改为 `draw_music_page`。

需要重新应用的功能：
- **封面形状支持**：`cover_shape` 参数（方形/圆形裁剪）
- **封面旋转动画**：`COVER_ROTATION`、`cover_rotate`、`dt` 参数
- **封面翻转动画**：`COVER_FLIP_*`、`trigger_cover_flip()`
- **封面缓存键修正**：`get_cached_media_image_with_key()` 函数
- **封面位置调整**：`img_x` 偏移
- **播放按钮尺寸**：`btn_scale` 调整

#### 4. `src/ui/expanded/widget_view.rs`

需要重新应用的功能：
- **Widget 页面歌词显示**：完整歌词区域绘制逻辑
- **歌词滚动动画**：`LyricScrollState` 结构体和 smoothstep 缓动
- **当前行长歌词滚动**：`CurrentLineScrollState` 结构体
- **歌词视觉层次**：当前行放大、其他行渐隐、指数衰减
- **歌词区域布局**：居中、边距、行高
- **dt 参数**：驱动滚动动画

#### 5. `src/window/settings.rs`

需要重新应用的功能：
- **设置界面主题系统**：`is_light` 字段、`SettingsTheme` 集成、`ThemeChanged` 事件处理
- **主题切换按钮**：`PopupKind::SettingsTheme`、`update_theme()` 方法
- **封面形状设置**：`MiniCoverShape`、`ExpandedCoverShape` popup
- **封面旋转开关**：`cover_rotate` 开关
- **音频门控开关**：`audio_gate` 开关
- **子选项卡**：外观/效果/行为三个子页面（`draw_sub_tabs`）
- **字体预览**：`FontPreview` 项目
- **灵动岛风格选项**：mica、dynamic 选项
- **设置窗口单实例**：`bring_settings_to_front()`（main.rs 已保留）
- **窗口大小调整**：`win_w`、`win_h` 动态尺寸
- **窗口尺寸**：WIN_W=666、WIN_H=666
- **窗口居中修复**：DPI 缩放因子考虑
- **分页逻辑优化**：按板块分页
- **性能优化**：items 缓存、hover 阈值、60fps
- **Popup 选项更新**：切换后调用 `mark_items_dirty()`
- **开关索引同步**：`sync_switch_targets` 包含 cover_rotate、audio_gate

**注意**：上游新增了 Dock 位置设置，需要确保用户的新设置项与上游的 Dock 设置项可以共存。

#### 6. `src/utils/settings_ui/renderer.rs`

需要重新应用的功能：
- **SettingsTheme 支持**：所有绘制函数新增 `theme: &SettingsTheme` 参数
- **FontPreview 绘制**：字体对比预览渲染逻辑
- **所有颜色引用**：从硬编码 `COLOR_*` 改为 `theme.*`

**注意**：上游对 renderer 做了性能优化（减少分配、缓存），需要将用户改动与上游优化合并。

#### 7. `src/utils/settings_ui/input.rs`

需要重新应用的功能：
- **RowLabel 处理**：点击检测中跳过 RowLabel 类型
- **idx 索引修正**：上游已用 `enumerate()` 重构，此改动可能不再需要

---

## 重要提示

### 编译注意事项

1. `main_view.rs` 已重命名为 `music_view.rs`，所有对 `main_view` 模块的引用需要更新
2. 上游 `draw_island` 已改用 `DrawIslandParams` 命名参数结构体
3. 上游 `app.rs` 新增了 `PluginManager` 字段，用户版本需要加入
4. `Cargo.toml` 新增了 workspace、plugin 相关依赖

### 验证步骤建议

1. 先尝试 `cargo build` 看是否能编译通过
2. 根据编译错误逐个修复引用问题
3. 从 `UPSTREAM_MERGE_IMPACT.md` 的"需要手动重新应用"清单中按优先级恢复功能
4. 先恢复核心功能（config 字段对应的 UI），再恢复视觉优化

### 功能优先级建议

| 优先级 | 功能 | 原因 |
|--------|------|------|
| P0 | config 新增字段对应的设置 UI | 否则设置界面无法显示/修改这些配置 |
| P0 | draw_island 新增参数 | 否则编译失败 |
| P0 | module 引用更新（main_view → music_view） | 否则编译失败 |
| P1 | 音频门控联动逻辑 | 核心功能 |
| P1 | 封面形状/旋转 | 视觉核心功能 |
| P1 | 设置界面主题 | 用户体验 |
| P2 | 歌词显示优化 | 视觉优化 |
| P2 | 自动隐藏行为修复 | 交互优化 |
| P3 | 设置界面性能优化 | 已有上游优化 |
| P3 | 其他视觉微调 | 锦上添花 |
