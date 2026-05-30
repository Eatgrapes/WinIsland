# 插件开发指南

欢迎！(´｡• ᵕ •｡`)♡ 你将通过插件扩展 WinIsland 的功能。

> ⚠️ **注意：插件系统目前处于基础阶段。** C ABI 类型定义已经就绪，但宿主端的 trait 接口（`ContentProvider`、`ThemeProvider`、`ShortcutProvider`）**尚未接入渲染管线**。请关注 [issue #55](https://github.com/Eatgrapes/WinIsland/issues/55) —— 非常期待听到你的想法！

## 插件工作原理

WinIsland 使用 **C ABI vtable** 模式来安全加载原生 `.dll` 插件：

```
WinIsland.exe  ──libloading──▶  your_plugin.dll
   │                                  │
   │  PluginManager                   │  导出 plugin_get_instance()
   │  └─ Vec<NativePlugin>            │  返回 PluginInstanceC {
   │       ├─ metadata (id, name…)    │    handle: 不透明指针
   │       ├─ handle (不透明指针)      │    vtable: 函数指针表
   │       └─ vtable (函数指针表)      │    metadata: PluginMetadataC
   │                                  │  }
   └── 调用 trait ──▶  通过 vtable ──▶  你的代码运行！
```

跨 FFI 边界的所有数据都是 `#[repr(C)]` — 扁平结构体，没有 `Vec`、`String` 或 trait 对象。这意味着你的插件可以用任意 Rust 版本编译都能正常工作 (ﾉ◕ヮ◕)ﾉ*:･ﾟ✧

## 插件类型（计划中）

| 类型 | 用途 | 状态 |
|------|------|------|
| **Content** (id=1) | 提供自定义岛内容（天气、通知、状态…） | 🔲 API 待定 |
| **Theme** (id=2) | 覆盖岛的配色和动画参数 | 🔲 API 待定 |
| **Shortcut** (id=3) | 注册可执行操作 | 🔲 API 待定 |

## 项目初始化

创建一个新的 Rust 库项目：

```
cargo new --lib my-winisland-plugin
```

编辑 `Cargo.toml`：

```toml
[package]
name = "my-winisland-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
winisland-plugin-api = "0.1"
```

## 实现插件

创建一个导出 C ABI 入口点的最小插件。

**src/lib.rs:**

```rust
use winisland_plugin_api::*;

// 插件实例就是你的插件状态。
struct MyPlugin;

// 唯一且必需的入口点 —— WinIsland 通过 libloading 调用它。
#[no_mangle]
pub extern "C" fn plugin_get_instance() -> PluginInstanceC {
    let handle = Box::into_raw(Box::new(MyPlugin)) as PluginHandle;

    // vtable 是静态的 —— 只要 DLL 被加载它就存在。
    static VTABLE: PluginVTable = PluginVTable {
        on_load:    on_load,
        on_unload:  on_unload,
        destroy:    destroy,
        get_content: None,
        on_click:   None,
        on_expanded: None,
        supports_expand: None,
        get_colors: None,
        get_animations: None,
        get_shortcuts_count: None,
        get_shortcut_at: None,
        execute_shortcut: None,
    };

    PluginInstanceC {
        handle,
        metadata: PluginMetadataC {
            id:          str_to_fixed("my-plugin"),
            name:        str_to_fixed("My Plugin"),
            version:     str_to_fixed("0.1.0"),
            author:      str_to_fixed("you"),
            description: str_to_fixed("A minimal WinIsland plugin"),
        },
        vtable: &VTABLE,
        plugin_type: PluginType::Content as u32,
    }
}

unsafe extern "C" fn on_load(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

unsafe extern "C" fn on_unload(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

unsafe extern "C" fn destroy(handle: PluginHandle) {
    drop(unsafe { Box::from_raw(handle as *mut MyPlugin) });
}
```

## 一行命令打包 (ﾉ◕ヮ◕)ﾉ

`winisland-plugin-api` crate 带有一个可选的 **packager** 模块，可自动完成编译、签名和 ZIP 打包。

### 1. 添加打包脚本

在 `Cargo.toml` 中添加：

```toml
[dev-dependencies]
winisland-plugin-api = { version = "0.1", features = ["packager"] }

[[bin]]
name = "pack"
path = "package.rs"
```

在项目根目录创建 `package.rs`：

```rust
fn main() {
    winisland_plugin_api::packager::PluginPackager::from_cargo()
        .unwrap()
        .signing_key_path("signing_key.pem")  // 可选
        .include_dir("assets")                 // 可选
        .build()
        .unwrap();
}
```

### 2. 一键构建

```bash
# 这一条命令完成：编译 + 签名（如果有密钥）+ 打包为 ZIP：
cargo run --bin pack
# 输出：target/my-winisland-plugin-0.1.0.zip
```

Packager 会自动：

1. 执行 `cargo build --release` 编译你的 DLL
2. 在 `target/release/` 中找到编译好的 `.dll`
3. 复制任何额外目录（如 `assets/`）
4. 计算所有 DLL 的 SHA-256 哈希
5. 用 Ed25519 密钥签名 manifest（如果提供了密钥）
6. 生成包含所有元数据的 `plugin.yml`
7. 打包为 `<name>-<version>.zip`

### 不使用 packager（手动打包）

插件必须以 `.zip` 格式打包才能被 WinIsland 加载。ZIP 必须包含：

```
my-plugin.zip
├── plugin.yml    ← 插件清单（必需）
└── *.dll         ← 插件二进制（必需，可多个 .dll）
```

#### plugin.yml

```yaml
name: example
author: xxx
version: 1.0.0
description: This is example plugin
github-link: example/example-plugin
```

**所有 5 个字段都是必需的** —— 缺少任何一个都会导致安装失败 o(TヘTo)

## 数字签名（推荐）

插件可以**选择性地**用 Ed25519 签名以验证真实性。签名后的插件在加载前会被校验——未签名的插件仍然可以工作，但会显示警告。

### 生成签名密钥

```bash
openssl genpkey -algorithm ed25519 -out signing_key.pem
openssl pkey -in signing_key.pem -pubout -out public_key.pem
```

`public_key.pem` 由项目维护者嵌入 WinIsland 二进制文件中。如果你是插件开发者，将通过安全渠道从 WinIsland 团队获取签名密钥。

### 打包时签名

```bash
cargo run --bin pack
```

如果 `signing_key.pem` 在项目根目录存在，packager 会自动签名插件。签名会嵌入 `plugin.yml`：

```yaml
name: my-plugin
author: you
version: 1.0.0
description: My awesome plugin
github-link: you/my-plugin
signature: "abc123deadbeef..."    # Ed25519 签名（64 字节 hex）
dll_hashes:
  - "sha256hashofdll..."
```

### CI 中使用环境变量签名

```yaml
# .github/workflows/release.yml
- run: cargo run --bin pack
  env:
    PLUGIN_SIGNING_KEY: ${{ secrets.PLUGIN_SIGNING_KEY }}
```

```rust
// package.rs
PluginPackager::from_cargo()
    .unwrap()
    .signing_key_env("PLUGIN_SIGNING_KEY")
    .build()
    .unwrap();
```

## 安装 ฅ^•ﻌ•^ฅ

直接把 **`.zip` 文件拖到岛（Dynamic Island）上**！插件会在后台线程中解压（保证岛保持流畅响应）并自动加载。

安装成功后会弹出 Windows 通知对话框确认。

你也可以手动将 `.dll` 文件放入插件目录的子目录中 —— WinIsland 启动时会扫描它们。

### 插件存储位置

```
C:\Users\<你的用户名>\AppData\Roaming\WinIsland\plugins\<插件名>\*.dll
```

## 安全性

WinIsland 在加载插件时应用了多重安全措施：

| 防护措施 | 说明 |
|---------|------|
| **插件 ID 校验** | ID 只能包含 `[a-zA-Z0-9_-]` |
| **ID 冲突检测** | 拒绝加载重复的插件 ID |
| **签名验证** | 加载前检查 Ed25519 签名（如果存在）|
| **路径穿越防护** | 拒绝包含 `..`、`:` 或绝对路径的 ZIP 条目 |
| **符号链接拒绝** | 拒绝 ZIP 中的符号链接条目 |
| **后台解压** | ZIP 解压在后台线程执行 |
| **锁中毒处理** | 锁中毒不会导致宿主崩溃 |
| **VTable 校验** | 调用前检查必需的函数指针是否为空 |

## 如何验证你的插件已加载？

由于宿主端 API 仍在开发中，插件暂时不会在岛界面上显示任何内容。

**验证方法：**
1. 按 `F12` 打开 WinIsland 调试日志窗口
2. 搜索你的插件名称——你应该能看到类似 `Loaded plugin: xxx (xxx)` 的信息
3. 拖放 ZIP 会触发 Windows 弹出窗口确认成功/失败

## C ABI 类型参考

这些类型定义在 `winisland-plugin-api` crate 中。在 API 集成完成前，它们仅用于 DLL 加载验证——**插件功能尚未在 UI 中可见**。

### PluginResultC

```rust
pub struct PluginResultC {
    pub ok: bool,
    pub error: [u8; 256],  // 以 null 结尾的 UTF-8
}
```

成功返回 `PluginResultC::ok()`，失败返回 `PluginResultC::err("消息")`。

### PluginMetadataC

```rust
pub struct PluginMetadataC {
    pub id: [u8; 64],
    pub name: [u8; 128],
    pub version: [u8; 32],
    pub author: [u8; 128],
    pub description: [u8; 256],
}
```

### IslandContentC

```rust
pub struct IslandContentC {
    pub tag: u32,
    pub title: [u8; 256],
    pub artist: [u8; 256],
    pub cover_url: [u8; 512],
    pub is_playing: bool,
    pub message: [u8; 256],
    pub label: [u8; 128],
    pub value: [u8; 128],
}
```

### PluginVTable

```rust
pub struct PluginVTable {
    pub on_load: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub on_unload: unsafe extern "C" fn(PluginHandle) -> PluginResultC,
    pub destroy: unsafe extern "C" fn(PluginHandle),
    pub get_content: Option<unsafe extern "C" fn(PluginHandle) -> IslandContentC>,
    pub on_click: Option<unsafe extern "C" fn(PluginHandle)>,
    pub on_expanded: Option<unsafe extern "C" fn(PluginHandle, bool)>,
    pub supports_expand: Option<unsafe extern "C" fn(PluginHandle) -> bool>,
    pub get_colors: Option<unsafe extern "C" fn(PluginHandle) -> ThemeColorsC>,
    pub get_animations: Option<unsafe extern "C" fn(PluginHandle) -> AnimationConfigC>,
    pub get_shortcuts_count: Option<unsafe extern "C" fn(PluginHandle) -> u32>,
    pub get_shortcut_at: Option<unsafe extern "C" fn(PluginHandle, i: u32, out: *mut ShortcutC)>,
    pub execute_shortcut: Option<unsafe extern "C" fn(PluginHandle, id: *const c_char) -> PluginResultC>,
}
```

### PluginInstanceC

```rust
pub struct PluginInstanceC {
    pub handle: PluginHandle,
    pub metadata: PluginMetadataC,
    pub vtable: *const PluginVTable,
    pub plugin_type: u32, // 1=Content, 2=Theme, 3=Shortcut
}
```

### ThemeColorsC / AnimationConfigC / ShortcutC

```rust
pub struct ThemeColorsC {
    pub primary: [u8; 4],    // RGBA
    pub secondary: [u8; 4],
    pub background: [u8; 4],
    pub text: [u8; 4],
    pub border: [u8; 4],
}

pub struct AnimationConfigC {
    pub expand_duration_ms: u32,
    pub collapse_duration_ms: u32,
    pub bounce_intensity: f32,
}

pub struct ShortcutC {
    pub id: [u8; 64],
    pub name: [u8; 128],
    pub description: [u8; 256],
    pub icon: [u8; 256],
    pub hotkey: [u8; 32],
}
```

## 加入讨论 (づ｡◕‿‿◕｡)づ

除了接入岛上下文之外，我们还没有太多具体的方向…… **真的很缺灵感 QWQ**

请来 [#55](https://github.com/Eatgrapes/WinIsland/issues/55) 一起讨论你希望插件系统支持什么功能！

---

Happy hacking! (づ｡◕‿‿◕｡)づ 如果遇到问题，欢迎在 [GitHub](https://github.com/Eatgrapes/WinIsland) 上开 issue。
