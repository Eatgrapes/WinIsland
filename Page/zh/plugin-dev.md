# 插件开发指南

欢迎！(´｡• ᵕ •｡`)♡ 你正准备为 WinIsland 开发自己的插件。

> ⚠️ **注意：插件系统目前是基础框架阶段**。C ABI 类型定义已经就绪，但 Host 端的 trait 接口（`ContentProvider`、`ThemeProvider`、`ShortcutProvider`）**尚未接入渲染管线**。详见 [issue #55](https://github.com/Eatgrapes/WinIsland/issues/55)，非常建议去那里讨论灵感！

## 插件工作原理

WinIsland 使用 **C ABI vtable** 模式来安全地加载原生 `.dll` 插件。可以这样理解：

```
WinIsland.exe  ──libloading──▶  your_plugin.dll
   │                                  │
   │  PluginManager                   │  导出 plugin_get_instance()
   │  └─ Vec<NativePlugin>            │  返回 PluginInstanceC {
   │       ├─ metadata (id, name…)    │    handle: 不透明指针
   │       ├─ handle (不透明指针)       │    vtable: 函数指针表
   │       └─ vtable (函数指针)         │    metadata: PluginMetadataC
   │                                  │  }
   └── 调用 trait ──▶  通过 vtable ──▶  你的代码运行！
```

所有跨 FFI 边界的数据都是 `#[repr(C)]` 的扁平结构体，没有 `Vec`、`String` 或 trait object。这意味着你的插件可以**用任意版本的 Rust 编译都能正常运行** (ﾉ◕ヮ◕)ﾉ*:･ﾟ✧

## 插件类型（计划中）

| 类型 | 作用 | 状态 |
|------|------|------|
| **Content** (id=1) | 提供自定义岛屿内容（天气、通知、状态…） | 🔲 API 待接入 |
| **Theme** (id=2) | 覆盖岛屿颜色和动画参数 | 🔲 API 待接入 |
| **Shortcut** (id=3) | 注册可执行动作 | 🔲 API 待接入 |

## 项目搭建

新建一个 Rust 库项目：

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
winisland-plugin-api = { git = "https://github.com/Eatgrapes/WinIsland", branch = "master" }
```

## 打包为 ZIP (ﾉ◕ヮ◕)ﾉ

你的插件需要打包成 `.zip` 格式才能被 WinIsland 加载。ZIP 包内至少包含：

```
my-plugin.zip
├── plugin.yml    ← 插件说明（必选）
└── *.dll         ← 插件本体（必选，可包含多个 .dll，所有 .dll 都会被加载）
```

### plugin.yml

```yaml
name: example
author: xxx
version: 1.0.0
description: This is example plugin
github-link: example/example-plugin
```

**全部 5 个字段缺一不可**，否则安装失败 o(TヘTo)

## 安装插件 ฅ^•ﻌ•^ฅ

只需将 `.zip` 文件**拖放到 WinIsland 的岛上**即可！插件会被解压到 `C:\Users\<你的用户名>\AppData\Roaming\WinIsland\plugins\<插件名>\`，并自动加载。

加载成功后会弹出 Windows 通知框确认。

你也可以手动将 `.dll` 放入 `plugins\` 文件夹下的子目录，WinIsland 启动时会自动扫描。

## 如何确认插件已加载？

由于 Host 端 API 还在开发中，插件暂时不会在 Island UI 上显示任何内容。

**验证方式：**
1. 按 `F12` 打开 WinIsland 调试日志窗口
2. 搜索你的插件名，应该能看到类似 `Loaded plugin: xxx (xxx)` 的日志
3. 拖入 ZIP 后会有 Windows 弹出框提示安装成功/失败

算是个雏形吧 (｡･ω･｡)

## C ABI 类型参考

以下类型都在 `winisland-plugin-api` crate 中。API 接入前这些类型仅用于 DLL 加载验证，**插件的实际功能尚未在 UI 中生效**。

### PluginResultC

```rust
pub struct PluginResultC {
    pub ok: bool,
    pub error: [u8; 256],  // 以 \0 结尾的 UTF-8
}
```

成功用 `PluginResultC::ok()`，失败用 `PluginResultC::err("错误信息")`。

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

### ThemeColorsC / AnimationConfigC

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
```

## 参与讨论 (づ｡◕‿‿◕｡)づ

目前除了接入灵动岛的 context 之外还没什么确定的方向…… **真的没什么灵感啊 QWQ**

非常欢迎到 [#55](https://github.com/Eatgrapes/WinIsland/issues/55) 讨论你希望插件系统支持什么功能！

---

祝开发愉快！(づ｡◕‿‿◕｡)づ 遇到问题欢迎到 [GitHub](https://github.com/Eatgrapes/WinIsland) 提 issue。
