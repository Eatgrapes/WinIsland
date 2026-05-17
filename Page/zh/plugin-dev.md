# 插件开发指南

欢迎！(´｡• ᵕ •｡`)♡ 你正准备为 WinIsland 开发自己的插件。本指南将带你了解一切。

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

## 插件类型

你可以写三种插件：

| 类型 | 作用 | 使用到的 VTable 字段 |
|------|------|-------------------|
| **Content** (id=1) | 提供自定义岛屿内容（天气、通知、状态…） | `get_content`, `on_click`, `on_expanded`, `supports_expand` |
| **Theme** (id=2) | 覆盖岛屿颜色和动画参数 | `get_colors`, `get_animations` |
| **Shortcut** (id=3) | 注册可执行动作 | _（vtable 中暂未暴露）_ |

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
winisland-plugin-api = { git = "https://github.com/oneachina/WinIsland", branch = "feat/plugin-system" }
```

## 打包为 ZIP (ﾉ◕ヮ◕)ﾉ

你的插件需要打包成 `.zip` 格式才能被 WinIsland 加载。ZIP 包内至少包含：

```
my-plugin.zip
├── plugin.yml    ← 插件说明（必选）
└── *.dll         ← 插件本体（必选，支持多个 .dll）
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

只需将 `.zip` 文件**拖放到 WinIsland 的岛上**即可！悬停时会显示 "📦 放入 zip~ 以加载插件"，松开鼠标后自动解压并加载，然后显示 "✅ 已加载 {name}~"

插件会被解压到 `C:\Users\<你的用户名>\AppData\Roaming\WinIsland\plugins\<插件名>\`。

## 编写一个 ContentProvider 插件

下面是一个最小示例——显示点击次数的插件 (｡･ω･｡)：

```rust
use std::ffi::c_void;
use winisland_plugin_api::*;

struct HelloPlugin {
    clicks: u32,
}

// ── VTable 回调 ──

extern "C" fn hello_on_load(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

extern "C" fn hello_on_unload(_handle: PluginHandle) -> PluginResultC {
    PluginResultC::ok()
}

extern "C" fn hello_destroy(handle: PluginHandle) {
    unsafe { drop(Box::from_raw(handle as *mut HelloPlugin)); }
}

extern "C" fn hello_get_content(handle: PluginHandle) -> IslandContentC {
    let plugin = unsafe { &mut *(handle as *mut HelloPlugin) };
    let mut label = [0u8; 128];
    let msg = format!("已点击 {} 次", plugin.clicks);
    let bytes = msg.as_bytes();
    let len = bytes.len().min(127);
    label[..len].copy_from_slice(&bytes[..len]);

    IslandContentC {
        tag: ISLAND_CONTENT_TAG_STATUS,
        label,
        ..zero_content()
    }
}

extern "C" fn hello_on_click(handle: PluginHandle) {
    let plugin = unsafe { &mut *(handle as *mut HelloPlugin) };
    plugin.clicks += 1;
}

extern "C" fn hello_supports_expand(_handle: PluginHandle) -> bool {
    false
}

// ── VTable ──

static VTABLE: PluginVTable = PluginVTable {
    on_load: hello_on_load,
    on_unload: hello_on_unload,
    destroy: hello_destroy,
    get_content: Some(hello_get_content),
    on_click: Some(hello_on_click),
    on_expanded: None,
    supports_expand: Some(hello_supports_expand),
    get_colors: None,
    get_animations: None,
};

// ── 元信息 ──

fn fill_metadata() -> PluginMetadataC {
    let mut id = [0u8; 64];
    let mut name = [0u8; 128];
    let mut version = [0u8; 32];
    let mut author = [0u8; 128];
    let mut description = [0u8; 256];

    write_str(&mut id, "hello_plugin");
    write_str(&mut name, "Hello Plugin");
    write_str(&mut version, "0.1.0");
    write_str(&mut author, "You! o(TヘTo)");
    write_str(&mut description, "A friendly example plugin");

    PluginMetadataC { id, name, version, author, description }
}

fn write_str(buf: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(buf.len() - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
}

fn zero_content() -> IslandContentC {
    IslandContentC {
        tag: 0,
        title: [0u8; 256],
        artist: [0u8; 256],
        cover_url: [0u8; 512],
        is_playing: false,
        message: [0u8; 256],
        label: [0u8; 128],
        value: [0u8; 128],
    }
}

// ── 入口 ──

#[unsafe(no_mangle)]
pub extern "C" fn plugin_get_instance() -> PluginInstanceC {
    let plugin = Box::new(HelloPlugin { clicks: 0 });
    PluginInstanceC {
        handle: Box::into_raw(plugin) as PluginHandle,
        metadata: fill_metadata(),
        vtable: &VTABLE,
        plugin_type: 1, // Content
    }
}
```

### 关键要点

1. **只导出 `plugin_get_instance`** —— 这是 WinIsland 唯一查找的符号。
2. **`handle` 是不透明的** —— 你拥有它的真实类型；Host 端永远不会直接碰它。
3. **填充所有 C 结构体字段** —— 未初始化的字节是 UB。使用 `zero_content()` 或 `[0u8; N]`。
4. **`destroy` 必须释放 handle** —— `Box::from_raw(handle)` 回收所有权，让 `drop` 执行。
5. **不需要的 vtable 字段填 `None`** —— 你的插件类型不支持的字段放心设为 `None`。

## IslandContentC 标签对照

返回 `IslandContentC` 时，设置 `tag` 为以下值之一：

| 标签常量 | 值 | 含义 |
|---|---|---|
| `ISLAND_CONTENT_TAG_MUSIC` | 1 | 填充 `title`, `artist`, `cover_url`, `is_playing` |
| `ISLAND_CONTENT_TAG_NOTIFICATION` | 2 | 填充 `title`, `message`, `cover_url`（作为图标） |
| `ISLAND_CONTENT_TAG_STATUS` | 3 | 填充 `label`, `value`, `cover_url`（作为图标） |

## C ABI 类型参考

所有类型都在 `winisland-plugin-api` crate 中，均为 `#[repr(C)]`。

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

所有字符串均为以 `\0` 结尾的 UTF-8。

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

具体哪些字段生效取决于 `tag`。不用的字段请填零。

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

## 排错指南 (╥﹏╥)

| 问题 | 检查 |
|---|---|
| 插件没有被加载 | `.dll` 放对目录了吗？查看 WinIsland 日志有没有 "Failed to load plugin" |
| Null handle / null vtable | `plugin_get_instance()` 必须填充所有字段 |
| 启动时崩溃 | 确保所有 C 结构体字段都用 `zero_content()` 或 `[0u8; N]` 初始化 |
| `get_content` 返回错误的数据 | 检查 `tag` 值和实际填充的字段是否匹配 |

---

祝开发愉快！(づ｡◕‿‿◕｡)づ 遇到问题欢迎到 [GitHub](https://github.com/Eatgrapes/WinIsland) 提 issue。
