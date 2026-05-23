# WinIsland 项目理解文档

## 重要要求
- **不要编译**：AI 助手无法在此环境中成功编译项目，请用户自行编译验证
- **代码最小化改动**：改动时候尽量减少对项目代码的修改，保持与原始项目一致
---

## 一、项目概述

**WinIsland** 是一个在 Windows 桌面上显示 **灵动岛（Dynamic Island）** 的桌面应用，灵感来源于 iPhone 14 Pro 的灵动岛设计。它能够实时监听系统中正在播放的音乐媒体，在屏幕顶部以动画胶囊的形式展示歌曲信息、专辑封面、频谱可视化、歌词等内容，并支持交互操作（播放/暂停、切歌、进度拖拽等）。

- **项目名称**: WinIsland
- **版本**: 1.0.0
- **作者**: Eatgrapes
- **许可证**: MIT
- **状态**: WIP（开发中）

---

## 二、技术栈

### 2.1 核心语言与运行时
| 技术 | 用途 |
|------|------|
| **Rust (Edition 2024)** | 主开发语言 |
| **Tokio** | 异步运行时（多线程、sync、time） |

### 2.2 图形与渲染
| 依赖 | 用途 |
|------|------|
| **skia-safe 0.93.1** | 2D 图形渲染引擎（绘制圆角矩形、文字、图片、模糊滤镜、渐变等） |
| **softbuffer 0.4.8** | 软件渲染缓冲区（将 skia 渲染结果写入窗口 surface） |
| **winit 0.30** | 跨平台窗口管理与事件循环 |
| **bytemuck** | 安全的字节转换（skia 像素数据 → softbuffer 缓冲区） |

### 2.3 音频处理
| 依赖 | 用途 |
|------|------|
| **cpal 0.15** | 跨平台音频 I/O（捕获系统音频输出用于频谱分析） |
| **realfft 3.3** | FFT 变换（将 PCM 音频数据转换为频域频谱） |

### 2.4 Windows 系统集成
| 依赖 | 用途 |
|------|------|
| **windows 0.58** | Windows API 绑定（SMTC 媒体控制、注册表自启动、GDI 截屏、窗口样式等） |
| **tray-icon 0.19** | 系统托盘图标与菜单 |

### 2.5 数据与网络
| 依赖 | 用途 |
|------|------|
| **serde + toml** | 配置文件序列化/反序列化（`~/.winisland/config.toml`） |
| **reqwest** | HTTP 客户端（歌词获取、自动更新检查） |
| **serde_json** | JSON 解析（歌词 API 响应、版本信息） |
| **tokio-util** | CancellationToken（优雅关闭异步任务） |

### 2.6 其他工具
| 依赖 | 用途 |
|------|------|
| **image** | 图像加载与处理（图标、缩略图） |
| **rfd** | 文件对话框（自定义字体选择） |
| **dirs** | 跨平台目录路径（获取 home 目录） |
| **once_cell** | 全局静态初始化（HTTP 客户端、i18n 实例） |
| **open** | 打开 URL/文件 |
| **winres** (build) | Windows 资源嵌入（图标、版本信息、manifest） |

### 2.7 文档网站
| 技术 | 用途 |
|------|------|
| **VitePress** | 项目文档网站（`Page/` 目录） |
| **GitHub Pages** | 文档部署 |
| **GitHub Actions** | CI/CD（自动构建 + Nightly 发布 + 文档部署） |

---

## 三、项目架构

```
WinIsland/
├── src/
│   ├── main.rs              # 入口：单实例检测、启动主窗口或设置窗口
│   ├── core/                # 核心业务逻辑
│   │   ├── config.rs        # AppConfig 配置结构体与默认值
│   │   ├── persistence.rs   # 配置文件读写（TOML）
│   │   ├── smtc.rs          # Windows SMTC 媒体会话监听与控制
│   │   ├── audio.rs         # 音频捕获与 FFT 频谱分析
│   │   ├── lyrics.rs        # 歌词获取（网易云163 / LRCLIB）
│   │   ├── i18n.rs          # 国际化（中/英）
│   │   └── render.rs        # 灵动岛主渲染逻辑
│   ├── ui/
│   │   └── expanded/        # 展开态 UI
│   │       ├── main_view.rs # 主页面（封面、标题、进度条、控制按钮、频谱）
│   │       └── widget_view.rs # 小组件页面（设置入口齿轮图标）
│   ├── icons/               # SVG 矢量图标绘制
│   │   ├── arrows.rs        # 箭头图标
│   │   ├── controls.rs      # 播放/暂停/上一首/下一首图标
│   │   ├── music.rs         # 音乐相关图标
│   │   └── settings.rs      # 设置图标
│   ├── window/              # 窗口管理
│   │   ├── app.rs           # 主应用窗口（灵动岛窗口 + 事件循环）
│   │   ├── settings.rs      # 设置窗口（独立进程）
│   │   └── tray.rs          # 系统托盘
│   └── utils/               # 工具模块
│       ├── physics.rs       # 弹簧物理动画
│       ├── anim.rs          # 动画值池（平滑过渡）
│       ├── blur.rs          # 运动模糊计算
│       ├── color.rs         # 颜色常量与工具
│       ├── font.rs          # 字体管理器（自定义字体、回退、缓存）
│       ├── glass.rs         # 毛玻璃效果（GDI 截屏 + Skia 模糊）
│       ├── scroll.rs        # 文字滚动动画
│       ├── mouse.rs         # 鼠标位置与按键检测
│       ├── icon.rs          # 应用图标加载
│       ├── autostart.rs     # 注册表开机自启动
│       ├── updater.rs       # 自动更新检查与热替换
│       └── settings_ui/     # 设置界面 UI 组件系统
│           ├── mod.rs
│           ├── items.rs     # 设置项数据模型
│           ├── renderer.rs  # 设置项渲染器
│           ├── input.rs     # 输入处理
│           └── anim.rs      # 设置界面动画
├── resources/               # 资源文件
│   ├── in_app/lang/         # 多语言文件（en.lang, zh.lang）
│   ├── icon.png / icon-dark.png / icon-dark.ico  # 应用图标
│   └── info-en.png / info-zh.png  # README 展示图
├── Page/                    # VitePress 文档网站
├── .github/workflows/       # CI/CD
├── Cargo.toml               # Rust 项目配置
└── build.rs                 # 构建脚本（嵌入 Windows 资源）
```

---

## 四、核心功能详解

### 4.1 灵动岛窗口（主窗口）

**文件**: [app.rs](src/window/app.rs)

- 使用 `winit` 创建**透明、无边框、置顶、跳过任务栏**的窗口
- 通过 Win32 API 设置 `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` 确保窗口不抢焦点、不显示在任务栏
- 窗口位置：屏幕顶部居中，支持多显示器选择和偏移量调整
- **单实例运行**：通过 `CreateMutexW` 确保只有一个主进程

#### 动画系统
- **弹簧物理** (`Spring`)：宽度、高度、圆角、视图切换、隐藏进度均使用弹簧动画
- **运动模糊**：根据弹簧速度计算模糊 sigma 值，在动画过程中添加方向性模糊
- **动画值池** (`AnimPool`)：用于设置界面中各种 UI 元素的平滑过渡

#### 交互
- **点击展开/收起**：点击灵动岛展开为详细视图
- **拖拽隐藏**：向下拖拽可隐藏灵动岛（保留小把手）
- **播放控制**：展开后可点击播放/暂停、上一首/下一首
- **进度条拖拽**：支持拖拽进度条跳转播放位置
- **视图切换**：主页面 ↔ 小组件页面（含设置入口）

### 4.2 SMTC 媒体监听

**文件**: [smtc.rs](src/core/smtc.rs)

- 使用 Windows **System Media Transport Controls (SMTC)** API 监听系统媒体会话
- 轮询间隔 300ms，同时监听 COM 事件实现即时响应
- **应用过滤**：只响应用户允许的应用（首次运行自动检测并添加音乐类应用）
- **媒体信息获取**：标题、艺术家、专辑、播放状态、播放位置、时长、缩略图
- **播放控制**：播放/暂停、上一首/下一首、跳转进度
- **缩略图获取**：通过 `Thumbnail.OpenReadAsync()` 获取专辑封面，带重试机制

### 4.3 音频频谱分析

**文件**: [audio.rs](src/core/audio.rs)

- 使用 `cpal` 捕获系统音频输出（loopback）
- 使用 `realfft` 进行 1024 点 FFT 变换
- 将频谱分为 6 个频段，自适应归一化
- **音频门控**：通过 Windows `IAudioMeterInformation` 检测是否有实际音频播放，避免静音时显示频谱
- 频段映射经过重新排列以获得更好的视觉效果

### 4.4 歌词系统

**文件**: [lyrics.rs](src/core/lyrics.rs)

- **歌词源**：
  - **网易云音乐 (163)**：搜索 → 匹配艺术家 → 获取歌词
  - **LRCLIB**：精确匹配 → 模糊搜索
- **备选源**：主源失败时自动切换到备选源
- **LRC 解析**：支持 `[mm:ss.ms]` 格式，支持翻译歌词合并
- **歌词延迟**：可配置偏移量（毫秒级）
- **歌词滚动**：迷你态歌词超长时可水平滚动

### 4.5 渲染管线

**文件**: [render.rs](src/core/render.rs), [main_view.rs](src/ui/expanded/main_view.rs)

1. skia-safe 在内存 Surface 上绘制
2. 绘制圆角矩形背景（黑色 / 毛玻璃风格）
3. 根据展开进度混合迷你态和展开态
4. 迷你态：专辑封面缩略图 + 频谱可视化 + 歌词
5. 展开态：专辑封面（带翻转动画）+ 标题/艺术家 + 进度条 + 控制按钮 + 频谱
6. 通过 `bytemuck` 将像素数据复制到 softbuffer 缓冲区
7. softbuffer 呈现到窗口

#### 毛玻璃效果
**文件**: [glass.rs](src/utils/glass.rs)

- 使用 GDI `BitBlt` 截取屏幕内容
- 使用 skia `image_filters::blur` 进行高斯模糊
- 叠加半透明黑色遮罩
- 带缓存机制（100ms 过期）

### 4.6 设置窗口

**文件**: [settings.rs](src/window/settings.rs)

- 独立进程运行（`--settings` 参数启动）
- 完全自绘 UI（skia + softbuffer），无传统控件
- 三个标签页：常规 / 音乐 / 关于
- 设置项类型：开关、步进器、下拉选择、字体选择器、应用列表
- 支持滚动、悬浮高亮、弹出菜单

### 4.7 自动更新

**文件**: [updater.rs](src/utils/updater.rs)

- 从 GitHub Releases 的 nightly 标签检查更新
- 比较本地与远程 `version_info.json` 的时间戳
- 更新流程：下载新 exe → 保存为 `.exe.new` → PowerShell 脚本等待旧进程退出 → 替换 → 重启

### 4.8 国际化 (i18n)

**文件**: [i18n.rs](src/core/i18n.rs)

- 支持中文和英文
- 自动检测系统语言（通过 `GetUserDefaultLocaleName`）
- 使用 `.lang` 文件（`key=value` 格式）
- 编译时内嵌 fallback 文本

---

## 五、配置系统

**配置路径**: `~/.winisland/config.toml`

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `global_scale` | f32 | 1.0 | 全局缩放 |
| `base_width` | f32 | 120.0 | 迷你态宽度 |
| `base_height` | f32 | 27.0 | 迷你态高度 |
| `expanded_width` | f32 | 360.0 | 展开态宽度 |
| `expanded_height` | f32 | 200.0 | 展开态高度 |
| `adaptive_border` | bool | false | 自适应边框颜色 |
| `motion_blur` | bool | true | 动态模糊效果 |
| `island_style` | String | "default" | 灵动岛风格（default/glass） |
| `smtc_enabled` | bool | true | 启用 SMTC 媒体控制 |
| `smtc_apps` | Vec<String> | [] | 允许的媒体应用列表 |
| `smtc_known_apps` | Vec<String> | [] | 已知媒体应用列表 |
| `show_lyrics` | bool | true | 显示歌词 |
| `custom_font_path` | Option<String> | None | 自定义字体路径 |
| `auto_start` | bool | false | 开机自启动 |
| `auto_hide` | bool | false | 自动隐藏 |
| `auto_hide_delay` | f32 | 5.0 | 自动隐藏延迟（秒） |
| `check_for_updates` | bool | true | 检查更新 |
| `update_check_interval` | f32 | 4.0 | 更新检查间隔（小时） |
| `language` | String | "auto" | 语言（auto/en/zh） |
| `lyrics_source` | String | "163" | 歌词源（163/lrclib） |
| `lyrics_fallback` | bool | true | 歌词备选源 |
| `lyrics_delay` | f64 | 0.0 | 歌词延迟（秒） |
| `lyrics_scroll` | bool | false | 歌词滚动 |
| `lyrics_scroll_max_width` | f32 | 300.0 | 歌词滚动最大宽度 |
| `position_x_offset` | i32 | 0 | 水平位置偏移 |
| `position_y_offset` | i32 | 0 | 垂直位置偏移 |
| `monitor_index` | i32 | 0 | 显示器索引 |
| `font_size` | f32 | 0.0 | 字体大小（0=自动） |

---

## 六、CI/CD 流程

**文件**: [.github/workflows/rust.yml](.github/workflows/rust.yml)

1. **构建 Rust 项目**（Windows runner）
   - 安装 Rust 工具链 + Ninja
   - `cargo build --release`
   - 生成 `version_info.json`（北京时间戳）
   - 删除旧的 nightly release
   - 上传 `WinIsland.exe` + `version_info.json` 到 GitHub Releases nightly 标签

2. **构建文档网站**（Ubuntu runner）
   - VitePress 构建
   - 部署到 GitHub Pages

---

## 七、设计亮点

1. **纯 Rust + Skia 自绘 UI**：不依赖任何 UI 框架（如 WebView、Qt、GTK），所有界面完全自绘，极致轻量
2. **弹簧物理动画**：所有尺寸变化使用弹簧系统，动画自然流畅
3. **运动模糊**：动画过程中添加方向性模糊，模拟真实运动效果
4. **毛玻璃效果**：通过 GDI 截屏 + Skia 模糊实现实时毛玻璃背景
5. **SMTC 深度集成**：利用 Windows 原生媒体控制 API，兼容所有支持 SMTC 的播放器
6. **音频频谱可视化**：实时 FFT 频谱分析，6 频段自适应归一化
7. **双歌词源**：网易云音乐 + LRCLIB，带自动回退
8. **热更新**：无需手动下载，应用内自动检查并替换更新
9. **多语言**：中英双语，自动检测系统语言

---

## 八、重要需求记录

> 此部分用于记录项目开发过程中的重要需求和决策。

### 8.1 当前已知限制
- 项目仍处于 WIP 状态，可能存在 bug
- 音频捕获依赖系统音频输出 loopback，某些音频驱动可能不兼容
- 毛玻璃效果通过 GDI 截屏实现，在高 DPI 或多显示器场景下可能有性能影响

### 8.2 优化需求清单

> **重要原则**：这是优化已存在的项目，之后要提交合并请求，遵循**最小化改动原则**，尽量不更改源文件代码结构。每次操作记录具体改动及必要性。

#### 小改动1：右滑界面歌词显示
- **问题**：右滑进入 widget_view 页面后只有左箭头和齿轮图标，没有任何歌词显示
- **目标**：在 widget_view 页面添加歌词显示区域，根据可用空间自动选择显示行数，当前播放行居中且动态放大
- **改动文件**：`src/ui/expanded/widget_view.rs`、`src/core/render.rs`
- **改动记录**：
  1. `widget_view.rs`：函数签名新增 `media: &MediaInfo` 和 `font_size: f32` 参数；新增歌词显示逻辑——根据可用高度计算最大可见行数，当前播放行居中、字体放大1.25倍并加粗，非当前行根据距离渐隐
  2. `render.rs`：调用 `draw_widget_page` 时传入 `media` 和 `font_size` 参数
- **必要性**：原 widget_view 无歌词功能，需要传入媒体信息才能获取歌词数据

#### 小改动2：设置界面跟随系统亮暗主题
- **问题**：设置界面使用硬编码深色主题颜色，不随系统主题变化
- **目标**：检测系统亮/暗主题，亮色时使用浅色配色，暗色时使用当前深色配色。只改设置界面的前端界面色彩
- **改动文件**：`src/utils/color.rs`、`src/window/settings.rs`、`src/utils/settings_ui/renderer.rs`
- **改动记录**：
  1. `color.rs`：新增 `SettingsTheme` 结构体（包含 win_bg/sidebar_bg/group_bg/card_highlight/text_pri/text_sec/disabled/accent/danger/toggle_on/toggle_off/sidebar_sel/sidebar_hover/separator/popup_bg/popup_border/hover_row/scrollbar 共18个颜色字段）；新增 `dark_settings_theme()` 和 `light_settings_theme()` 两个构造函数，暗色主题复用原有常量，亮色主题使用 iOS 风格浅色配色
  2. `settings.rs`：`SettingsApp` 结构体新增 `is_light: bool` 字段；新增 `theme()` 方法根据 `is_light` 返回对应主题；`draw()` 方法中获取主题并传递给 `draw_sidebar/draw_items/draw_popup`；`draw_sidebar` 签名改为接受 `&SettingsTheme`，内部所有硬编码颜色替换为 `theme.*`；`draw_popup` 同理；`resumed` 中通过 `window.theme()` 检测初始主题；`window_event` 中新增 `ThemeChanged` 事件处理，更新 `is_light` 并触发重绘
  3. `renderer.rs`：`draw_items` 签名新增 `theme: &SettingsTheme` 参数；`draw_switch/draw_stepper_btn/draw_row_hover` 内部函数签名均新增 `theme: &SettingsTheme`；所有 `COLOR_*` 常量和 `color_*()` 函数调用替换为 `theme.*` 字段访问
- **必要性**：原代码所有颜色硬编码为深色主题常量，无法支持亮色主题。通过 `SettingsTheme` 结构体抽象颜色，使得设置界面可以根据系统主题切换配色，同时保持原有常量不变（灵动岛主界面仍使用深色）

#### 大改动1：SMTC 封面切歌后不更新
- **问题**：切换歌曲后封面永远显示上一首的封面，整首歌都不切换，错位严重
- **根因分析**：`smtc.rs` 中 `fetch_properties` 函数的缩略图获取逻辑存在两个问题：
  1. 歌曲切换时立即启动异步获取缩略图，但 Windows SMTC session 的媒体属性更新有延迟，`TryGetMediaPropertiesAsync()` 可能返回旧歌曲的属性和缩略图
  2. 异步任务中没有验证获取到的媒体属性（标题/艺术家）是否与预期匹配，导致旧封面被当作新封面设置
- **目标**：确保切歌后封面能及时更新为新歌曲的封面
- **改动文件**：`src/core/smtc.rs`
- **改动记录**：
  1. 缩略图异步获取任务中，当检测到歌曲切换（`is_song_change = should_fetch_lyrics`）时，先等待 800ms 让 SMTC 更新属性
  2. 在每次重试获取缩略图时，先读取 `props.Title()` 和 `props.Artist()`，与预期的 `title_clone`/`artist_clone` 比对，不匹配则视为"过期属性"跳过本次，继续重试
  3. 重试间隔优化：前3次间隔300ms（快速重试），后续间隔500ms
- **必要性**：SMTC 属性更新有延迟是 Windows API 的已知行为，必须在获取缩略图前验证属性一致性，否则会获取到旧封面

#### 大改动2：封面拉伸变形 → 居中裁剪
- **问题**：非正方形的封面图被 `draw_image_rect_with_sampling_options` 拉伸到正方形区域（72×72×scale），导致变形
- **目标**：改为居中裁剪（cover fit）方式，保持图片原始比例，居中裁剪填充正方形区域
- **改动文件**：`src/ui/expanded/main_view.rs`、`src/core/render.rs`
- **改动记录**：
  1. `main_view.rs`：封面绘制前计算源矩形 `src_rect`——获取图片宽高比，宽图裁左右留中、高图裁上下留中，生成居中正方形裁剪区域；`draw_image_rect_with_sampling_options` 的第二个参数从 `None` 改为 `src_rect.as_ref()`
  2. `render.rs`：迷你态封面绘制同样计算源矩形实现居中裁剪，逻辑与展开态一致
- **必要性**：原代码将任意比例图片拉伸到正方形，导致非正方形封面变形。居中裁剪保持图片比例，虽然会丢失边缘信息但视觉效果远优于拉伸

#### 补充改动1：歌词显示优化
- **问题**：歌词显示位置偏左，未完全显示；字体缩放不适合 UI 风格
- **改动文件**：`src/ui/expanded/widget_view.rs`
- **改动记录**：
  1. 歌词区域边距调整：左边距从 28.0 改为 32.0，右边距从 16.0 改为 20.0，上边距从 16.0 改为 12.0，下边距从 36.0 改为 40.0
  2. 字体大小固定为 12.0 * scale（移除动态缩放逻辑）
  3. 行高从 1.8 倍改为 2.0 倍
  4. 当前行放大倍数从 1.25 改为 1.15
  5. 歌词居中：新增 `center_x` 变量计算歌词区域水平中心点，`draw_text_cached` 的位置参数从 `(lyric_area_left, line_y)` 改为 `(center_x, line_y)`
- **必要性**：原代码歌词以左边界为基准居中，导致歌词整体偏左；字体缩放逻辑过于复杂，固定大小更适合 UI 风格

#### 补充改动2：封面位置调整
- **问题**：封面位置太偏左，不符合灵动岛设计
- **改动文件**：`src/ui/expanded/main_view.rs`、`src/core/render.rs`
- **改动记录**：
  1. `main_view.rs`：展开态封面 `img_x` 从 `ox + 24.0 * scale` 改为 `ox + 28.0 * scale`
  2. `render.rs`：迷你态封面 `ix` 从 `offset_x + 8.0 * global_scale` 改为 `offset_x + 10.0 * global_scale`
- **必要性**：封面偏左影响视觉平衡，向右移动后更符合灵动岛居中设计

#### 补充改动3：设置界面主题切换按钮
- **问题**：设置界面只能跟随系统主题，无法手动切换
- **目标**：添加主题切换下拉菜单，支持"跟随系统/浅色/深色"三个选项
- **改动文件**：`src/core/config.rs`、`src/window/settings.rs`、`resources/in_app/lang/en.lang`、`resources/in_app/lang/zh.lang`
- **改动记录**：
  1. `config.rs`：`AppConfig` 结构体新增 `settings_theme: String` 字段（默认值 "system"）；新增 `default_settings_theme()` 函数；`Default` 实现中添加该字段
  2. `settings.rs`：`PopupKind` 枚举新增 `SettingsTheme` 变体；`build_general_items` 中添加 `RowSourceSelect` 项目；`handle_click` 中添加 `PopupKind::SettingsTheme` 处理逻辑；新增 `update_theme()` 方法根据配置更新 `is_light` 字段
  3. `en.lang`：新增 `settings_theme/theme_system/theme_light/theme_dark` 翻译
  4. `zh.lang`：新增对应中文翻译
- **必要性**：用户可能希望设置界面使用与系统不同的主题，手动切换提供更大灵活性

#### 补充改动4：封面形状切换（方形/圆形）
- **问题**：封面只能显示为圆角方形，无法切换为圆形；圆形时视觉上比方形小
- **目标**：添加封面形状切换下拉菜单，支持"方形/圆形"两个选项；迷你态和展开态可独立设置；圆形时略微放大保持视觉平衡
- **改动文件**：`src/core/config.rs`、`src/window/settings.rs`、`src/core/render.rs`、`src/ui/expanded/main_view.rs`、`resources/in_app/lang/en.lang`、`resources/in_app/lang/zh.lang`
- **改动记录**：
  1. `config.rs`：将 `cover_shape` 字段拆分为 `mini_cover_shape` 和 `expanded_cover_shape` 两个独立字段（默认值均为 "square"）；新增 `default_mini_cover_shape()` 和 `default_expanded_cover_shape()` 函数
  2. `settings.rs`：`PopupKind` 枚举将 `CoverShape` 拆分为 `MiniCoverShape` 和 `ExpandedCoverShape`；`build_general_items` 中添加两个独立的 `RowSourceSelect` 项目；`handle_click` 中分别处理两种形状
  3. `render.rs`：`draw_island` 函数签名新增 `mini_cover_shape` 和 `expanded_cover_shape` 两个参数；迷你态封面圆形时放大 1.15 倍并调整位置居中
  4. `main_view.rs`：`draw_main_page` 函数签名参数名保持 `cover_shape`（传入 `expanded_cover_shape`）；展开态封面圆形时放大 1.08 倍并调整位置居中
  5. `app.rs`：调用 `draw_island` 时传入 `&self.config.mini_cover_shape` 和 `&self.config.expanded_cover_shape`
  6. `en.lang`/`zh.lang`：将 `cover_shape` 拆分为 `mini_cover_shape`（迷你态封面形状）和 `expanded_cover_shape`（展开态封面形状）
- **必要性**：迷你态和展开态的封面大小和上下文不同，独立设置更灵活；圆形面积比方形小约 21.5%，放大后视觉上更平衡

#### 补充改动5：新增 Mica 和动态配色背景风格
- **问题**：原毛玻璃效果通过 GDI 截屏实现，性能开销大
- **目标**：添加 Windows Mica API（Win11）和封面主色调动态配色两种高性能背景方案
- **改动文件**：`src/utils/backdrop.rs`（新增）、`src/utils/mod.rs`、`src/core/render.rs`、`src/window/settings.rs`、`src/window/app.rs`、`resources/in_app/lang/en.lang`、`resources/in_app/lang/zh.lang`
- **改动记录**：
  1. `backdrop.rs`（新增）：实现 `try_enable_mica()` 函数调用 Windows DWM API；实现 `get_dynamic_bg_color()` 从封面提取主色调并生成深色背景；实现 `get_text_color_for_bg()` 根据背景亮度返回合适的文字颜色
  2. `mod.rs`：添加 `pub mod backdrop;` 导出新模块
  3. `render.rs`：`draw_island` 中新增对 "mica" 和 "dynamic" 风格的处理；mica 使用半透明深色背景；dynamic 从封面提取主色调生成背景
  4. `settings.rs`：`island_style` 选项新增 "mica" 和 "dynamic" 两个选项
  5. `app.rs`：窗口创建时若配置为 mica 风格则调用 `try_enable_mica()`
  6. `en.lang`/`zh.lang`：新增 `style_mica` 和 `style_dynamic` 翻译
- **必要性**：Mica 是 Windows 11 原生效果，性能最优；动态配色无需截屏，性能好且视觉效果类似 Spotify 动态主题

#### 补充改动6：设置界面帧率优化
- **问题**：设置界面持续重绘导致 CPU 占用高
- **目标**：只在有动画或交互时才请求重绘
- **改动文件**：`src/utils/anim.rs`、`src/utils/settings_ui/anim.rs`、`src/window/settings.rs`
- **改动记录**：
  1. `anim.rs`：`AnimPool` 新增 `is_animating()` 方法检测是否有活动动画
  2. `settings_ui/anim.rs`：`SwitchAnimator` 新增 `is_animating()` 方法
  3. `settings.rs`：`about_to_wait` 中添加检查逻辑，只在 `has_anim || has_popup || is_scrolling` 时才继续处理和请求重绘
- **必要性**：避免无意义的 CPU 占用，降低功耗

#### 补充改动7：歌词显示视觉优化
- **问题**：当前播放行不够突出，其他行透明度不够明显
- **目标**：增强当前行的视觉突出效果，降低其他行的视觉权重
- **改动文件**：`src/ui/expanded/widget_view.rs`
- **改动记录**：
  1. 当前播放行：字体放大至 1.25 倍（原 1.15 倍），亮度 100%，使用粗体
  2. 其他行：字体缩小至 0.95 倍，透明度降低 40%（乘以 0.6），亮度降低至 70%
  3. 距离渐变：距离当前行越远透明度越低（fade 系数从 0.5 改为 0.7）
- **必要性**：让用户更容易识别当前播放的歌词行，提升阅读体验

#### 补充改动8：设置界面灵动岛风格选项不完整
- **问题**：设置界面"灵动岛风格"选项只显示"默认"和"毛玻璃"，缺少"云母"和"动态配色"选项
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. `build_items` 函数中 `RowSourceSelect` 的 `options` 数组，新增 `(tr("style_mica"), ...)` 和 `(tr("style_dynamic"), ...)` 两个选项
- **必要性**：新增的 Mica 和动态配色功能需要在设置界面提供选择入口

#### 补充改动9：歌词显示优化（第二轮）
- **问题**：歌词放大效果不明显，视觉不够突出；歌词没有视觉居中，与左右控制组件位置不对齐
- **改动文件**：`src/ui/expanded/widget_view.rs`
- **改动记录**：
  1. 当前行字体大小从 `base_font_size * 1.25` 改为固定 `36.0 * scale`（与控制按钮大小相近）
  2. 取消当前行加粗，改为普通字体 `FontStyle::normal()`
  3. 歌词区域左右边距从 32.0/20.0 改为 40.0/40.0，实现视觉居中
  4. 非当前行透明度从 0.6 降低到 0.5，亮度从 0.7 降低到 0.6
  5. 非当前行字体大小动态计算：`base_font_size * (1.0 - dist / (half + 1.0) * 0.3)`，距离当前行越远字体越小
- **必要性**：用户反馈歌词放大效果不明显，需要更突出的视觉层次；居中对齐与控制按钮位置一致

#### 补充改动10：动态颜色切换时保持上一个颜色
- **问题**：切换歌曲时，由于封面获取速度原因，动态配色会短暂变回黑色，造成视觉闪烁
- **改动文件**：`src/utils/backdrop.rs`、`src/core/render.rs`
- **改动记录**：
  1. `backdrop.rs`：新增 `LAST_VALID_COLOR` 线程本地缓存，存储最近一次成功提取的颜色
  2. `backdrop.rs`：`get_dynamic_bg_color` 函数成功提取颜色后，同时更新 `LAST_VALID_COLOR`
  3. `backdrop.rs`：新增 `get_last_valid_color()` 函数，返回最近一次有效的颜色
  4. `render.rs`：动态配色逻辑中，当无法获取封面图片时，使用 `get_last_valid_color()` 返回的颜色，避免黑色空挡
- **必要性**：封面获取有延迟时，使用上一个有效颜色可以保持视觉连续性，避免闪烁

#### 补充改动11：歌词显示优化（第三轮）
- **问题**：歌词显示效果仍不符合主流播放器设计，当前行过大、非当前行过暗
- **参考**：主流播放器（Spotify、Apple Music、YouTube Music）的歌词显示设计
- **改动文件**：`src/ui/expanded/widget_view.rs`
- **改动记录**：
  1. 统一字体大小为 `16.0 * scale`，当前行和非当前行使用相同基础大小
  2. 非当前行使用指数衰减公式：
     - 字体大小：`font_size * 0.96^dist`（相邻行为 96%，再远为 92%，以此类推）
     - 透明度：`opacity * 0.7^dist`（相邻行为 70%，再远为 49%，以此类推）
  3. 所有歌词使用纯白色（255, 255, 255），通过透明度区分层次
  4. 行高改为 `font_size * 2.0`，更舒适的阅读间距
- **必要性**：参考主流播放器设计，当前行保持正常大小和完全不透明，相邻行略微缩小和透明，形成自然的视觉层次

#### 补充改动12：删除未使用代码
- **问题**：`get_text_color_for_bg` 函数编译警告"never used"
- **改动文件**：`src/utils/backdrop.rs`
- **改动记录**：删除 `get_text_color_for_bg` 函数
- **必要性**：该函数是为将来功能预留的，但当前未使用，删除以消除编译警告

#### 补充改动13：歌词平滑滚动动画
- **问题**：歌词切换时是瞬间跳转，没有动画效果，与灵动岛其他动态效果不一致
- **目标**：为歌词切换添加平滑滚动动画，与灵动岛的弹簧动画风格一致
- **改动文件**：`src/ui/expanded/widget_view.rs`、`src/core/render.rs`、`src/window/app.rs`
- **改动记录**：
  1. `widget_view.rs`：新增 `LyricScrollState` 结构体，包含 `current_idx`、`old_idx`、`scroll_progress` 字段
  2. `widget_view.rs`：`update` 方法记录旧行索引，使用 `scroll_progress` 从 0 渐变到 1，速度 0.12/帧
  3. `widget_view.rs`：使用 smoothstep 缓动函数 `ease_progress = t*t*(3-2*t)` 实现平滑过渡
  4. `widget_view.rs`：`scroll_offset = idx_diff * line_h * (1.0 - ease_progress)` 计算滚动偏移
  5. `widget_view.rs`：动画时增加 `extra_lines = 3` 额外行数，确保过渡时上下行可见
  6. `widget_view.rs`：新当前歌词淡入（`fade = ease_progress`），旧当前歌词淡出（`fade = 1.0 - ease_progress`）
  7. `widget_view.rs`：`draw_widget_page` 函数返回 `bool` 表示是否正在动画中
  8. `render.rs`：`draw_island` 函数返回 `bool`，传递 `widget_animating` 状态
  9. `app.rs`：当 `widget_animating` 为 true 时调用 `window.request_redraw()` 请求重绘
- **必要性**：歌词滚动动画与灵动岛的弹簧动画风格一致，提升视觉体验的连贯性

#### 补充改动14：歌词区域布局优化
- **问题**：歌词区域中心没有对齐左侧箭头位置，视觉上不垂直居中
- **目标**：让歌词区域中心对齐切换组件位置，提升视觉一致性
- **改动文件**：`src/ui/expanded/widget_view.rs`
- **改动记录**：
  1. `lyric_area_bottom` 从 `oy + h - 40.0 * scale` 改为 `oy + h - 12.0 * scale`，不再预留齿轮图标空间
  2. `center_y` 从 `lyric_area_top + lyric_area_h / 2.0` 改为 `oy + h / 2.0`，对齐左侧箭头位置
  3. `center_x` 从 `lyric_area_left + lyric_area_w / 2.0` 改为 `ox + w / 2.0`，居中于整个方框
  4. **修复整数/浮点除法不一致问题**：`line_y` 计算中的 `extra_lines as f32 / 2.0` 改为 `(extra_lines / 2) as f32`，确保与索引计算一致
- **必要性**：歌词区域中心与切换组件对齐，视觉上更协调

#### 补充改动15：歌词当前行放大与动画优化
- **问题**：当前歌词行不够突出，动画帧率不够平滑，滚动方向反了
- **目标**：让当前行更醒目，动画更顺滑，滚动方向正确
- **改动文件**：`src/ui/expanded/widget_view.rs`
- **改动记录**：
  1. 当前歌词行字体大小从 `font_size` 改为 `font_size + 6.0 * scale`，放大6个字号
  2. 旧当前歌词行（动画中淡出）同样放大6个字号，保持过渡一致性
  3. 动画速度从 `0.12` 改为 `0.08`，让滚动更平滑
  4. **修复滚动方向**：`scroll_offset` 从 `idx_diff * line_h * (1.0 - ease_progress)` 改为 `-idx_diff * line_h * (1.0 - ease_progress)`
     - 原问题：切换到下一行时歌词向下跳，然后向上移动
     - 修复后：切换到下一行时歌词从下方移入，向上滚动到中心
- **必要性**：当前行更醒目，动画更流畅自然，滚动方向符合用户预期

#### 补充改动16：方框内当前行歌词滚动
- **问题**：方框内居中显示的当前行歌词如果过长会被截断，无法完整显示；歌词垂直位置未对齐左箭头按钮
- **目标**：当当前行歌词超过方框宽度时，自动滚动显示完整内容，借鉴迷你态滚动歌词实现；歌词垂直居中对齐左箭头按钮
- **改动文件**：`src/ui/expanded/widget_view.rs`、`src/core/render.rs`、`src/window/app.rs`
- **改动记录**：
  1. `widget_view.rs`：新增 `CurrentLineScrollState` 结构体，包含 `text_hash`、`offset`、`pause`、`direction` 字段
  2. `widget_view.rs`：`CurrentLineScrollState::update()` 方法实现滚动逻辑——文本变化时重置，溢出时来回滚动，两端暂停1.5秒
  3. `widget_view.rs`：`draw_widget_page` 函数新增 `dt` 参数，用于驱动滚动动画
  4. `widget_view.rs`：计算当前行文本宽度 `current_text_w`，判断是否溢出 `current_overflow`
  5. `widget_view.rs`：当前行滚动时使用 `text_x = lyric_area_left - current_scroll_offset` 定位，非滚动时居中显示
  6. `widget_view.rs`：返回值改为 `is_animating || is_current_scrolling`，滚动时持续触发重绘
  7. `widget_view.rs`：`center_y` 从 `oy + h / 2.0` 改为 `oy + h / 2.0 + 4.0 * scale`，修正文本基线偏移，视觉上垂直居中对齐左箭头
  8. `render.rs`：`draw_island` 函数新增 `dt` 参数，传递给 `draw_widget_page`
  9. `app.rs`：`RedrawRequested` 事件中计算 `dt`，传递给 `draw_island`
- **必要性**：长歌词可完整显示，歌词垂直居中对齐左箭头按钮，与迷你态滚动歌词体验一致，提升用户体验

#### 补充改动17：圆形封面旋转动画
- **问题**：圆形封面静态显示，缺乏动态效果，与其他音乐播放器体验不一致
- **目标**：为圆形封面添加旋转动画，播放时自动旋转，暂停时停止，可在设置中开关
- **改动文件**：`src/core/config.rs`、`src/ui/expanded/main_view.rs`、`src/core/render.rs`、`src/window/app.rs`、`src/window/settings.rs`、`resources/in_app/lang/zh.lang`、`resources/in_app/lang/en.lang`
- **改动记录**：
  1. `config.rs`：新增 `cover_rotate: bool` 配置项，默认值 `false`
  2. `main_view.rs`：新增 `COVER_ROTATION` thread_local 存储旋转角度
  3. `main_view.rs`：`draw_main_page` 函数新增 `cover_rotate` 和 `dt` 参数，返回 `bool` 表示是否正在动画
  4. `main_view.rs`：圆形封面绘制时，若 `cover_rotate && cover_shape == "circle" && is_playing` 则应用旋转变换
  5. `render.rs`：新增 `MINI_COVER_ROTATION` thread_local 存储迷你态旋转角度
  6. `render.rs`：`draw_island` 函数新增 `cover_rotate` 参数
  7. `render.rs`：迷你态圆形封面同样支持旋转动画
  8. `render.rs`：旋转时设置 `widget_animating = true` 触发持续重绘
  9. `app.rs`：`draw_island` 调用添加 `self.config.cover_rotate` 参数
  10. `settings.rs`：效果设置部分新增"封面旋转"开关
  11. `settings.rs`：更新开关索引（cover_rotate 为索引 2，后续索引顺延）
  12. `zh.lang`：新增 `cover_rotate=封面旋转`
  13. `en.lang`：新增 `cover_rotate=Cover Rotate`
- **必要性**：旋转动画是音乐播放器的经典视觉效果，提升用户体验和动态感

#### 补充改动18：歌词滚动与设置开关修复
- **问题**：方框内歌词滚动时第一个字被裁剪；封面旋转开关状态不同步
- **目标**：修复歌词滚动起始位置偏移，修复设置界面开关动画同步
- **改动文件**：`src/ui/expanded/widget_view.rs`、`src/window/settings.rs`
- **改动记录**：
  1. `widget_view.rs`：滚动歌词 `text_x` 从 `lyric_area_left - current_scroll_offset` 改为 `lyric_area_left + 2.0 * scale - current_scroll_offset`，添加左边距避免首字被裁剪
  2. `settings.rs`：`sync_switch_targets` 函数新增索引 2 对应 `cover_rotate`，后续索引顺延（3-9）
- **必要性**：修复视觉问题，确保设置界面开关状态正确同步

#### 补充改动19：设置界面性能优化
- **问题**：设置界面渲染时导致明显卡顿，影响主窗口（灵动岛）的流畅度
- **目标**：优化设置界面的重绘策略，减少不必要的计算和轮询
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. `settings.rs`：新增 `cached_items: Option<Vec<SettingsItem>>` 和 `items_dirty: bool` 字段
  2. `settings.rs`：`build_current_items` 改为缓存机制，只在 `items_dirty` 时重新构建
  3. `settings.rs`：新增 `mark_items_dirty` 方法，在配置变更和页面切换时调用
  4. `settings.rs`：`SwitchAnimator::new` 新增 `cover_rotate` 初始值
  5. `settings.rs`：`about_to_wait` 轮询间隔从 16ms 降为 33ms（约 30fps）
  6. `settings.rs`：重置默认时同步更新 `SwitchAnimator`
- **必要性**：减少设置界面的 CPU 占用，避免影响灵动岛的流畅渲染

#### 补充改动20：暂停状态隐藏歌词并显示控制组件
- **问题**：暂停时歌词仍然显示且进度不同步；缺少暂停状态下的交互控制
- **目标**：暂停时隐藏歌词、显示控制按钮（上一曲/播放/下一曲），歌词进度由 SMTC 自动同步
- **改动文件**：`src/core/render.rs`、`src/window/app.rs`
- **改动记录**：
  1. `render.rs`：新增 `MINI_PAUSE_ANIM` thread_local 存储暂停动画状态
  2. `render.rs`：导入 `draw_play_button`、`draw_pause_button`、`draw_control_triangle`
  3. `render.rs`：迷你态暂停时（`music_active && !media.is_playing`）隐藏歌词，显示控制按钮
  4. `render.rs`：控制按钮包含上一曲（左三角）、播放/暂停（中间）、下一曲（右三角）
  5. `render.rs`：新增 `get_mini_control_rects` 函数返回三个按钮的点击区域
  6. `app.rs`：导入 `get_mini_control_rects`
  7. `app.rs`：迷你态暂停时点击检测控制按钮区域，触发对应播放控制命令
- **必要性**：暂停时提供直观的交互控制，歌词进度由 SMTC 的 `position_ms` 自动维护（暂停时使用 `position_ms` 而非外推值）

---

## 待解决问题

### ~~问题1：设置界面卡顿~~ ✅ 已解决
- 已通过缓存 items 和降低轮询频率优化

### ~~问题2：暂停状态下歌词进度记录~~ ✅ 已解决
- 暂停时隐藏歌词，显示控制组件
- 歌词进度由 SMTC 的 `position_ms` 自动维护（`current_lyric` 方法在暂停时直接使用 `position_ms`）

### ~~问题3：暂停状态显示控制组件~~ ✅ 已解决
- 已在迷你态暂停时显示上一曲/播放/下一曲控制按钮

---

#### 补充改动21：暂停控制按钮尺寸修复 + 超时消失修复
- **问题**：暂停控制按钮中间播放按钮太大；暂停几秒后灵动岛组件全消失；控制按钮点击无效
- **根因分析**：
  1. `btn_scale = 0.7 * global_scale` 导致播放按钮视觉上远大于两侧三角
  2. `music_active` 使用 `last_playing_time.elapsed() < 5s` 超时逻辑，暂停超过5秒后 `music_active=false` → 岛进入空闲态 → 自动隐藏
  3. 点击无效是问题2的连锁反应：`is_paused` 依赖 `music_on`，超时后 `music_on=false` → 点击检测代码不执行
- **改动文件**：`src/core/render.rs`、`src/window/app.rs`
- **改动记录**：
  1. `render.rs`：`btn_scale` 从 `0.7 * global_scale` 改为 `0.45 * global_scale`，缩小中间播放按钮
  2. `app.rs`：三处 `music_active` 判断逻辑从 `is_playing || last_playing_time.elapsed() < 5s` 改为 `!media.title.is_empty()`，只要有歌曲标题就保持 `music_active=true`
  3. `app.rs`：`about_to_wait` 中的 `music_active` 同样改为 `!media.title.is_empty()`
- **必要性**：暂停时应该保持灵动岛显示，直到用户关闭音乐应用或切歌导致标题清空

#### 补充改动23：暂停态多项修复
- **问题**：暂停时歌词仍刷新；暂停岛显示上一首/下一首按钮多余；暂停时自动隐藏失效
- **根因分析**：
  1. 歌词刷新：`current_lyric_opt` 在暂停时仍调用 `media.current_lyric()`，导致歌词文本变化触发重绘
  2. 上一首/下一首多余：迷你暂停岛空间有限，只需居中播放/暂停按钮
  3. 自动隐藏失效：改动21将 `music_active` 改为 `!title.is_empty()`，导致 `is_idle` 在暂停时永远为 false，自动隐藏计时器永远不计时
- **改动文件**：`src/window/app.rs`、`src/core/render.rs`
- **改动记录**：
  1. `app.rs`：歌词获取增加 `!is_paused` 条件，暂停时不更新歌词文本
  2. `render.rs`：移除暂停态的上一首/下一首三角按钮绘制，只保留居中播放/暂停按钮
  3. `render.rs`：移除 `btn_gap` 变量（不再需要偏移）
  4. `app.rs`：点击处理只检测播放按钮，移除 prev/next 检测
  5. `app.rs`：`is_idle` 逻辑改为 `(!music_active || is_paused_idle)`，暂停时也允许自动隐藏计时
- **必要性**：暂停态体验优化，修复自动隐藏逻辑

#### 补充改动24：自动隐藏弹回修复 + 歌词冻结 + 设置流畅性
- **问题**：自动隐藏后立即被弹回；暂停时歌词进度丢失；设置界面交互卡顿
- **根因分析**：
  1. 自动隐藏弹回：`about_to_wait` 中 `music_active && auto_hidden` 条件在暂停时也为 true，岛一隐藏就被恢复
  2. 歌词丢失：改动23中 `!is_paused` 使 `current_lyric_opt=None`，触发 else 分支清空歌词文本
  3. 设置卡顿：鼠标每次 `CursorMoved` 都重建 items 并遍历 hover 检测；帧率仅 30fps
- **改动文件**：`src/window/app.rs`、`src/core/render.rs`、`src/window/settings.rs`
- **改动记录**：
  1. `app.rs`：自动隐藏恢复条件从 `music_active` 改为 `media.is_playing`，暂停时不再自动恢复岛
  2. `app.rs`：歌词清空条件增加 `!is_paused`，暂停时保留当前歌词文本（冻结）
  3. `render.rs`：暂停播放按钮 `btn_scale` 从 `0.45` 改为 `0.28`，再缩小
  4. `settings.rs`：新增 `last_hover_mouse_pos` 字段，鼠标移动 <0.5px 时跳过 hover 检测
  5. `settings.rs`：帧率从 33ms (30fps) 提升到 16ms (60fps)
  6. `settings.rs`：滚动插值系数从 `0.28` 提升到 `0.35`，滚动更跟手
- **必要性**：修复核心交互问题，提升设置界面响应速度

#### 补充改动25：暂停时歌词滚动停止
- **问题**：暂停后歌词仍在滚动，导致持续重绘
- **根因分析**：歌词滚动逻辑 `if overflow > 0.0 && lyric_transition >= 1.0` 在暂停时仍执行，持续更新 `lyric_scroll_offset` 并调用 `request_redraw()`
- **改动文件**：`src/window/app.rs`
- **改动记录**：
  1. `app.rs`：歌词滚动条件增加 `&& !is_paused`，暂停时停止滚动动画
- **必要性**：暂停时应该冻结所有歌词相关动画，避免不必要的重绘

#### 补充改动26：暂停时 position_ms 微小漂移过滤
- **问题**：暂停后歌词仍会刷新（短暂显示暂停行后变化）
- **根因分析**：
  1. SMTC 后台线程第521行：`smtc_changed && !is_playing` 条件在暂停时仍会同步 position 变化
  2. 暂停时 SMTC 的 position 可能有微小漂移（系统行为）
  3. 后台线程同步了漂移后的 position_ms
  4. 主线程虽然暂停时不调用 `current_lyric()`，但 position 已变
- **改动文件**：`src/core/smtc.rs`
- **改动记录**：
  1. `smtc.rs`：新增 `diff_from_last` 计算 position 与上次记录的差异
  2. `smtc.rs`：暂停时同步条件改为 `smtc_changed && !is_playing && diff_from_last > 500`
  3. 效果：暂停时只有 position 差异超过 500ms 才同步（过滤微小漂移，仍响应 seek 操作）
- **必要性**：暂停时应该冻结歌词进度，避免微小漂移导致歌词变化

---

### 歌词系统说明

#### 歌词获取流程
```
SMTC 后台线程                    主线程 (about_to_wait)
     │                                │
     ▼                                ▼
获取 title/artist              self.smtc.get_info()
获取 position_ms                       │
获取 is_playing                       ▼
     │                         media.current_lyric(delay_ms)
     ▼                                │
fetch_lyrics(title, artist,           ▼
  duration_secs, source,      二分查找当前歌词行
  fallback)                           │
     │                                ▼
     ▼                         返回歌词文本或 None
解析 LRC 格式
     │
     ▼
存储到 MediaInfo.lyrics
```

#### 歌词数据结构
```rust
pub struct LyricLine {
    pub time_ms: u64,    // 歌词行开始时间（毫秒）
    pub text: String,    // 歌词文本
}
```

#### 能否通过歌词获取歌曲信息？
**不能**。原因：
1. 歌词 API 需要 `title`, `artist`, `duration_secs` 作为输入参数
2. 歌词数据只包含时间轴和文本，不包含歌曲元信息
3. API 调用方向是：歌曲信息 → 歌词数据（不可逆）

#### 歌曲时长来源
- **SMTC TimelineProperties**：`session.GetTimelineProperties()?.End()`
- 这是系统媒体控制提供的官方时长，比歌词最后一行更准确

#### 补充改动27：暂停时停止持续重绘
- **问题**：暂停后时间轴仍在刷新
- **根因分析**：
  1. `about_to_wait` 第883行：`music_active` 条件在暂停时也为 true
  2. 暂停时每帧都调用 `request_redraw()`
  3. 每帧重绘时重新计算进度条和时间显示
  4. 即使暂停，UI 也在持续更新
- **改动文件**：`src/window/app.rs`
- **改动记录**：
  1. `app.rs`：重绘条件从 `music_active` 改为 `music_active && media.is_playing`
  2. 效果：暂停时停止持续重绘，只有播放时才持续更新时间轴
  3. 保留：展开态（expanded）和 spring 动画仍会触发重绘
- **必要性**：暂停时应该冻结所有动态更新，节省 CPU 资源

---

## 待解决问题

### 问题1：暂停时歌词/时间轴仍刷新（未完全解决）
- **现象**：暂停后歌词或时间轴仍会刷新
- **已尝试修复**：
  1. 改动23：暂停时不调用 `current_lyric()`
  2. 改动23：暂停时不清空歌词文本
  3. 改动25：暂停时停止歌词滚动动画
  4. 改动26：过滤 position_ms 微小漂移
  5. 改动27：暂停时停止持续重绘
- **仍存在问题**：可能还有其他触发重绘的路径未找到
- **待排查方向**：
  1. 检查是否有其他 `request_redraw()` 调用
  2. 检查 SMTC 后台线程是否仍在发送更新
  3. 检查 `draw_main_page` 中的动画状态

---

## 补充改动

### 补充改动28：设置界面 popup 选项切换后更新显示
- **问题**：主题切换后方框文字不更新
- **根因**：popup 选项切换后没有调用 `mark_items_dirty()`，导致缓存的 items 没有更新
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. 在 `save_config` 后添加 `self.mark_items_dirty()`
  2. 效果：所有 popup 选项切换后都会重建 items 缓存

### 补充改动30：设置窗口可调整大小
- **需求**：设置界面窗口可拖动调整大小
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. 添加 `win_w` 和 `win_h` 字段存储动态窗口大小
  2. 窗口创建时设置 `with_resizable(true)` 和 `with_min_inner_size()`
  3. 处理 `WindowEvent::Resized` 事件更新窗口大小
  4. 所有绘制和点击检测使用动态窗口大小
- **效果**：窗口可以拖动调整大小，最小尺寸为 680x480

### 补充改动31：设置界面边框计算修复
- **问题**：窗口大小改变后内容显示不完整
- **根因**：`win_w` 和 `win_h` 存储的是物理像素，但绘制时需要逻辑像素
- **改动文件**：`src/window/settings.rs`
- **改动记录**：在绘制时除以 scale 因子获取逻辑大小

### 补充改动32：设置界面分页显示
- **需求**：固定每页词条数量，左右滑动切换
- **改动文件**：
  1. `src/window/settings.rs`：添加分页字段和滑动逻辑
  2. `src/utils/settings_ui/renderer.rs`：添加 `draw_items_paged` 函数
- **改动记录**：
  1. 添加 `content_page`, `page_offset`, `total_pages` 等字段
  2. 根据窗口高度计算每页可显示的行数
  3. 滚轮和拖动实现页面切换
  4. 添加分页指示器（圆点）

### 补充改动34：切换歌曲时封面保持显示
- **问题**：切换歌曲时封面会空白
- **改动文件**：
  1. `src/ui/expanded/main_view.rs`：修改 `get_cached_media_image` 函数
  2. `src/window/app.rs`：歌曲切换时调用 `trigger_cover_flip()`
- **改动记录**：
  1. 当新封面不可用时，返回上一张封面（`COVER_FLIP_OLD_IMG`）
  2. 歌曲切换时触发封面翻转动画

### 补充改动35：扁平模式控制按钮点击
- **问题**：扁平模式下只有暂停时才能点击播放按钮
- **改动文件**：`src/window/app.rs`
- **改动记录**：
  1. 扁平模式下始终检测控制按钮区域
  2. 点击上一曲/播放/下一曲按钮分别触发对应操作
  3. 不再限制只有暂停时才能点击

### 补充改动36：设置窗口默认尺寸和居中
- **需求**：设置窗口默认尺寸改为1065x1073，并居中显示
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. 修改 `WIN_W` 和 `WIN_H` 常量为 1065.0 和 1073.0
  2. 获取主显示器尺寸计算居中位置
  3. 使用 `with_position` 设置窗口初始位置

### 补充改动37：设置界面分页逻辑优化
- **需求**：同一板块（Group）放一页，避免分割
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. 新增 `calculate_page_breaks` 函数，按板块计算分页断点
  2. 遍历 items 时检测 GroupStart/GroupEnd
  3. 整个 Group 放在同一页，避免跨页分割

### 补充改动38：设置界面点击检测修复
- **问题**：分页后内部选择按键无效果
- **根因**：点击检测未考虑分页偏移
- **改动文件**：
  1. `src/utils/settings_ui/input.rs`：新增 `hit_test_paged` 函数
  2. `src/utils/settings_ui/mod.rs`：导出新函数
  3. `src/window/settings.rs`：使用分页版点击检测
- **改动记录**：
  1. `hit_test_paged` 接收 `start_row` 和 `end_row` 参数
  2. 只检测当前页可见的行
  3. 三个 handle_*_click 函数都添加分页参数


#### 补充改动39：修复设置界面编译错误
- **问题**：设置界面点击选择按钮全部失效，编译时出现 `ClickResult::Select` 不存在错误
- **根因分析**：`src/window/settings.rs` 中 `handle_general_click` 方法存在重复的 `ClickResult::SourceButton` 处理块，导致编译错误
- **目标**：修复编译错误，使设置界面点击选择按钮正常工作
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. 删除重复的 `ClickResult::SourceButton` 处理块（第2个错误的处理块）
  2. 保留正确的 `ClickResult::SourceButton` 处理逻辑
- **必要性**：修复编译错误，使设置界面功能恢复正常，按照最小化改动原则只删除重复代码而不改变现有逻辑

#### 补充改动40：设置界面点击检测坐标修复
- **问题**：设置界面所有点击按钮（开关、选择框、步进器）全部失效
- **根因分析**：`handle_click` 和 `get_hover_state` 函数中 `content_w` 计算使用了 `self.config.global_scale`，但 `logical_mouse_pos` 是逻辑坐标（已除以 DPI 缩放），应使用 `win.scale_factor()`
- **目标**：修复点击检测坐标计算，使所有按钮正常响应
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. `handle_click` 中 `scale` 从 `self.config.global_scale` 改为 `self.window.as_ref().unwrap().scale_factor() as f32`
  2. `get_hover_state` 中同样修复 `scale` 计算
- **必要性**：点击坐标必须与绘制坐标使用相同的缩放基准，否则所有交互失效

#### 补充改动41：设置界面子选项卡功能
- **问题**：用户希望"外观"、"效果"、"行为"三个设置区域可以通过左右切换选项卡方式浏览
- **目标**：在"常规"页面下添加子选项卡，支持点击和键盘左右切换
- **改动文件**：`src/window/settings.rs`、`src/utils/settings_ui/items.rs`
- **改动记录**：
  1. `settings.rs`：新增 `SUB_TAB_H`、`SUB_TAB_START_Y` 常量
  2. `settings.rs`：`SettingsApp` 新增 `active_sub_page`、`sub_tab_hover` 字段
  3. `settings.rs`：新增 `draw_sub_tabs` 函数绘制子选项卡 UI
  4. `settings.rs`：修改 `build_general_items` 根据 `active_sub_page` 返回不同设置项
  5. `settings.rs`：修改 `draw` 函数调用 `draw_sub_tabs` 并调整内容区域起始位置
  6. `settings.rs`：修改 `handle_click` 添加子选项卡点击检测
  7. `settings.rs`：修改 `get_hover_state` 添加子选项卡 hover 检测
  8. `settings.rs`：修改键盘事件处理，左右箭头键在"常规"页面时切换子选项卡
  9. `settings.rs`：修改 `get_page_anim` 根据子选项卡返回正确的开关动画索引
  10. `settings.rs`：修改 `handle_general_click`、`handle_music_click` 开关索引映射
- **必要性**：提升设置界面用户体验，子选项卡比滚动浏览更直观

#### 补充改动42：设置界面行项目索引修复
- **问题**：方框选择按钮（RowSourceSelect）点击后 popup 弹出位置错误
- **根因分析**：`hit_test` 返回的 `idx` 是行项目索引，但 `handle_general_click` 中用 `items.iter().take(idx)` 累加高度时取的是前 `idx` 个所有项目（含非行项目），导致位置计算错误
- **目标**：修复行项目索引与实际位置的对应关系
- **改动文件**：`src/utils/settings_ui/items.rs`、`src/window/settings.rs`
- **改动记录**：
  1. `items.rs`：新增 `get_row_item(items, row_idx)` 辅助函数获取第 N 个行项目
  2. `settings.rs`：`SourceButton` 处理中高度累加改为遍历所有项目并只对行项目计数
  3. `settings.rs`：`StepperDec/StepperInc` 处理中 `items.get(idx)` 改为 `get_row_item(items, idx)`
  4. `settings.rs`：`SourceButton` 处理中 `items.get(idx)` 改为 `get_row_item(items, idx)`
- **必要性**：确保点击位置与 popup 弹出位置一致，所有交互功能正常

#### 补充改动43：设置窗口单实例检测
- **问题**：多次点击设置按钮会打开多个设置窗口
- **目标**：设置窗口只允许单实例，已有窗口时将其前置而非新开
- **改动文件**：`src/main.rs`、`src/window/settings.rs`
- **改动记录**：
  1. `main.rs`：添加 `Local\\WinIsland_Settings_Mutex` 单实例检测
  2. `settings.rs`：新增 `bring_settings_to_front()` 函数查找并前置已存在的设置窗口
  3. `settings.rs`：导入 `FindWindowW`、`SetForegroundWindow`、`ShowWindow`、`SW_RESTORE` Windows API
- **必要性**：避免多个设置窗口造成用户困惑和资源浪费

#### 补充改动44：设置窗口宽度调整
- **问题**：设置窗口宽度原为 1065.0，用户希望改为 666.0
- **目标**：调整设置窗口宽度为 666.0
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. `settings.rs`：`WIN_W` 常量从 1065.0 改为 666.0
- **必要性**：用户界面偏好调整

#### 补充改动45：字体对比预览功能
- **问题**：用户选择自定义字体后无法直观看到与默认字体的对比效果
- **目标**：在字体选择器下方添加字体对比预览区域，显示默认字体和自定义字体的中日英例句
- **改动文件**：`src/utils/settings_ui/items.rs`、`src/utils/settings_ui/renderer.rs`、`src/window/settings.rs`、`src/utils/font.rs`、`resources/in_app/lang/zh.lang`、`resources/in_app/lang/en.lang`
- **改动记录**：
  1. `items.rs`：新增 `FontPreview { has_custom_font: bool }` 类型
  2. `items.rs`：`height()` 方法返回 `FontPreview` 高度为 120.0
  3. `renderer.rs`：添加 `FontPreview` 绘制逻辑，左侧显示默认字体例句，右侧显示自定义字体例句
  4. `settings.rs`：在 `RowFontPicker` 后添加 `FontPreview` 项目
  5. `font.rs`：新增 `draw_text_with_custom_font()` 方法强制使用自定义字体绘制
  6. `zh.lang`：添加 `font_preview_default=默认字体`、`font_preview_custom=自定义字体`
  7. `en.lang`：添加 `font_preview_default=Default Font`、`font_preview_custom=Custom Font`
- **必要性**：让用户直观对比自定义字体效果，提升用户体验

#### 补充改动46：设置窗口高度调整
- **问题**：设置窗口高度原为 1073.0，用户希望改为 666.0
- **目标**：调整设置窗口高度为 666.0
- **改动文件**：`src/window/settings.rs`
- **改动记录**：
  1. `settings.rs`：`WIN_H` 常量从 1073.0 改为 666.0
- **必要性**：用户界面偏好调整

#### 补充改动47：动态主题和封面形状切换刷新优化
- **问题**：切换动态主题或封面形状后，界面没有立即刷新，需要等待较长时间才能看到效果
- **根因分析**：
  1. 配置检测间隔为 60 帧（约 1 秒），切换后需要等待较长时间
  2. 切换 `island_style` 时动态配色缓存 `DYNAMIC_BG_CACHE` 没有被清除
  3. 切换封面形状时封面缓存 `IMG_CACHE` 没有被清除
- **目标**：优化配置更新后的刷新速度，确保样式切换后立即生效
- **改动文件**：`src/window/app.rs`、`src/utils/backdrop.rs`、`src/ui/expanded/main_view.rs`
- **改动记录**：
  1. `app.rs`：配置检测间隔从 60 帧改为 30 帧（约 0.5 秒）
  2. `app.rs`：配置更新时检测 `island_style` 变化并调用 `clear_dynamic_bg_cache()`
  3. `app.rs`：配置更新时检测封面形状变化并调用 `clear_cover_cache()`
  4. `backdrop.rs`：新增 `clear_dynamic_bg_cache()` 函数清除动态配色缓存
  5. `main_view.rs`：新增 `clear_cover_cache()` 函数清除封面缓存
- **必要性**：确保样式切换后立即生效，提升用户体验

#### 补充改动48：设置窗口随主程序退出关闭
- **问题**：灵动岛退出时设置窗口不会一起关闭
- **根因**：设置窗口检测主程序 mutex 的间隔为 60 帧（约 1 秒），退出时可能检测不到
- **改动文件**：`src/window/settings.rs`
- **改动记录**：将 mutex 检测间隔从 60 帧改为 30 帧（约 0.5 秒）
- **必要性**：确保设置窗口在主程序退出后能及时关闭

#### 补充改动49：歌曲切换时动态颜色更新
- **问题**：动态颜色获取会慢一首歌，切换歌曲后颜色还是显示上一首歌的颜色
- **根因分析**：
  1. 当歌曲切换时，`media.title` 和 `media.album` 立即更新
  2. 但 `media.thumbnail` 可能还没加载完成（异步加载）
  3. `get_cached_media_image` 在新封面不可用时返回旧封面（`COVER_FLIP_OLD_IMG`）
  4. 但调用方用新的 `cache_key`（新歌曲标题）来缓存颜色
  5. 结果：新歌曲标题 -> 旧封面图片 -> 错误的颜色被缓存
- **改动文件**：`src/ui/expanded/main_view.rs`、`src/core/render.rs`、`src/window/app.rs`
- **改动记录**：
  1. `main_view.rs`：新增 `get_cached_media_image_with_key()` 函数，返回 `(Image, String)` 元组，包含实际使用的缓存键
  2. `main_view.rs`：`get_cached_media_image()` 改为调用新函数并只返回图片
  3. `main_view.rs`：`get_media_palette()` 使用新函数获取正确的缓存键
  4. `main_view.rs`：`draw_main_page()` 使用新函数获取正确的缓存键
  5. `render.rs`：`draw_island()` 使用 `get_cached_media_image_with_key()` 获取正确的缓存键
  6. `app.rs`：歌曲切换时调用 `clear_dynamic_bg_cache()` 清除缓存
- **必要性**：确保歌曲切换后动态颜色能立即更新，且颜色与封面图片一致

#### 补充改动50：设置窗口居中修复
- **问题**：设置窗口打开时不在屏幕正中央，而是偏右下角
- **根因**：`monitor.size()` 返回的是物理像素，但 `LogicalPosition` 需要的是逻辑像素，没有考虑 DPI 缩放因子
- **改动文件**：`src/window/settings.rs`
- **改动记录**：获取显示器 `scale_factor`，将物理像素转换为逻辑像素后再计算居中位置
- **必要性**：确保设置窗口在所有 DPI 设置下都能正确居中显示

#### 补充改动51：删除语言文件中的右键重命名文字
- **问题**：删除昵称重命名功能后，语言文件中仍保留"右键重命名"的文字
- **改动文件**：`resources/in_app/lang/zh.lang`、`resources/in_app/lang/en.lang`
- **改动记录**：
  1. `zh.lang`：`media_apps=媒体应用程序（右键重命名）` 改为 `media_apps=媒体应用程序`
  2. `en.lang`：`media_apps=MEDIA APPLICATIONS (Right-click to rename)` 改为 `media_apps=MEDIA APPLICATIONS`
- **必要性**：保持界面文字与实际功能一致

#### 补充改动4：字体预览对比效果修复
- **问题**：设置界面效果选项中，当用户选择自定义字体后，左边的"默认字体"预览也使用了自定义字体，导致左右两边字体相同，无法形成对比效果
- **目标**：左边的"默认字体"预览始终显示系统默认字体，右边的"自定义字体"预览显示用户选择的字体，形成对比效果
- **改动文件**：`src/utils/font.rs`、`src/utils/settings_ui/renderer.rs`
- **改动记录**：
  1. `font.rs`：新增 `draw_text_with_default_font` 方法，强制使用系统默认字体（Microsoft YaHei / Segoe UI），不使用自定义字体
  2. `renderer.rs`：`FontPreview` 绘制中，左边的标签和示例文字从 `fm.draw_text()` 改为 `fm.draw_text_with_default_font()`，确保始终显示默认字体
- **必要性**：原代码 `draw_text` 方法会优先使用自定义字体，导致"默认字体"预览无法正确展示系统默认字体

#### 补充改动52：音频会话静音修复（修正版）
- **问题**：软件启动时以满音量运行，导致Wallpaper壁纸音频被占用，播放断断续续
- **根因分析**：
  1. `cpal` 创建音频输入流（loopback 捕获系统音频）时，Windows 会为该流创建新的音频会话
  2. 新创建的音频会话默认音量为最大值（1.0）
  3. 原代码使用 `SetMasterVolume(0.0, ...)` 设置音量为 0，但这不是真正的静音
  4. 音量为 0 时，系统仍然认为有音频会话在运行，Wallpaper Engine 等软件会暂停自己的音频输出，导致播放断断续续
- **目标**：软件启动时不占用系统音量资源，保持永久静音状态，同时不影响频谱可视化功能
- **改动文件**：`src/core/audio.rs`
- **改动记录**：
  1. 修复 `CoCreateInstance` 调用语法（使用 `.ok()` 模式匹配）
  2. 修复 `GetSimpleAudioVolume` 参数类型（第二个参数从 `None` 改为 `0`）
  3. 将 `session.SetMasterVolume(0.0, std::ptr::null())` 改为 `session.SetMute(true, std::ptr::null())`
  4. 使用 `SetMute(true, ...)` 真正静音音频会话，系统不会将此进程视为正在播放音频的进程
- **效果**：解决了Wallpaper壁纸播放音频断断续续的问题
- **必要性**：`SetMasterVolume(0.0, ...)` 只是设置音量为 0，系统仍认为有音频会话在运行；`SetMute(true, ...)` 才是真正的静音，系统不会将此进程视为音频播放者

#### 补充改动53：音频门控开关与隐藏状态联动
- **问题**：音频门控（`gate`）机制通过 `IAudioMeterInformation` 检测系统音频峰值，当没有音频播放时 `gate=0`，导致频谱值全部为 0（`raw_bins[j] * gate = 0`），频谱动画条不显示。即使用户正在播放音乐，灵动岛可见时频谱也可能因门控检测延迟而不显示
- **用户需求**：
  1. 在设置中添加按钮控制音频门控功能的开启/关闭
  2. 默认开启：隐藏时静音（gate=0），不隐藏时开启（gate=1）
- **改动文件**：`src/core/config.rs`、`src/core/audio.rs`、`src/window/app.rs`、`src/window/settings.rs`、`resources/in_app/lang/zh.lang`、`resources/in_app/lang/en.lang`
- **改动记录**：
  1. `config.rs`：新增 `audio_gate: bool` 配置项，默认值 `true`
  2. `audio.rs`：新增 `gate_override: Arc<AtomicU32>` 字段，默认值 `1.0`；新增 `set_gate_override(value: bool)` 方法；频谱计算中 `gate` 改为 `gate * gate_override`（`effective_gate`）
  3. `app.rs`：每帧获取频谱后，根据 `audio_gate` 配置和隐藏状态调用 `set_gate_override`：
     - `audio_gate=true` 且未隐藏 → `set_gate_override(true)` → `effective_gate = gate * 1.0`（正常门控）
     - `audio_gate=true` 且已隐藏 → `set_gate_override(false)` → `effective_gate = gate * 0.0`（强制静音）
     - `audio_gate=false` → `set_gate_override(false)` → `effective_gate = gate * 0.0`（完全禁用门控，频谱始终为 0）
  4. `settings.rs`：在「效果」区域添加「音频门控」开关；更新 `SwitchAnimator` 索引（插入第 3 位，后续索引 +1）
  5. `zh.lang`：添加 `audio_gate=音频门控`
  6. `en.lang`：添加 `audio_gate=Audio Gate`
- **必要性**：原始代码中 `gate` 值完全由 `IAudioMeterInformation` 检测决定，无法外部控制，导致频谱动画在静音检测下失效。通过 `gate_override` 机制，用户可以在设置中控制门控功能，同时实现「隐藏时静音，显示时开启」的智能行为