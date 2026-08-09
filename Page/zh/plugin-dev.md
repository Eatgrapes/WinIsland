# 插件开发指南

WinIsland 插件 API `0.3` 定义了原生 ABI v1。插件是直接加载进 WinIsland 进程的受信任 Windows DLL，没有沙箱或崩溃隔离，只应安装可信来源的插件。

旧版 `0.2` 的 `PluginVTable`、`PluginType`、`plugin_get_instance` 和 `plugin_set_host_api` 均不再兼容。

## 架构

```text
插件 DLL 导出 winisland_plugin_entry_v1()
    -> PluginDescriptorV1
    -> WinIsland 校验 ABI、能力和元数据
    -> WinIsland 签发 PluginToken 并调用 create(PluginCreateInfoV1)
    -> 插件查询版本化的 Context/Media/I18n/HostState 服务
    -> 插件创建由宿主管理、以 ResourceId 标识的资源
    -> WinIsland 调用 shutdown(handle)
    -> WinIsland 回收剩余资源
    -> WinIsland 调用 destroy(handle) 并卸载 DLL
```

生命周期严格为 `create -> shutdown -> destroy`。`shutdown` 必须同步停止并 join 所有可能执行插件代码或调用宿主服务的线程。只有 `shutdown` 成功后，WinIsland 才会销毁 handle 并卸载 DLL。

## 项目配置

```toml
[package]
name = "hello-winisland-plugin"
version = "0.1.0"
edition = "2024"
authors = ["Example Author"]
description = "Minimal WinIsland ABI v1 plugin"
repository = "https://github.com/example/hello-winisland-plugin"

[lib]
name = "hello_winisland_plugin"
crate-type = ["cdylib"]

[dependencies]
winisland-plugin-api = "0.3"
```

API crate 的 README 提供了一个完整的 Context 插件，可直接作为最初的 `src/lib.rs`。

## 入口描述符

每个插件只导出一个入口：

```rust
#[unsafe(no_mangle)]
/// # Safety
/// WinIsland 使用 ABI v1 约定的签名调用该函数。
pub unsafe extern "C" fn winisland_plugin_entry_v1() -> *const PluginDescriptorV1 {
    &DESCRIPTOR
}
```

描述符必须具有静态生命周期：

```rust
static DESCRIPTOR: PluginDescriptorV1 = PluginDescriptorV1 {
    struct_size: std::mem::size_of::<PluginDescriptorV1>() as u32,
    abi_version: ABI_VERSION_1,
    capabilities: CAPABILITY_CONTEXT | CAPABILITY_HOST_STATE,
    metadata: PluginMetadataC::new(
        "hello-winisland-plugin",
        "hello-winisland-plugin",
        "0.1.0",
        "Example Author",
        "Minimal WinIsland ABI v1 plugin",
    ),
    create: Some(create),
    shutdown: Some(shutdown),
    destroy: Some(destroy),
};
```

插件 ID 必须匹配 `[a-zA-Z0-9_-]+`。安装包和 DLL 描述符中的元数据必须完全一致。

## 查询宿主服务

在 `create` 中校验 `PluginCreateInfoV1`，保存 `plugin_token`，并查询已声明的服务：

```rust
let info = unsafe { &*create_info };
let host = unsafe { &*info.host_api };
let context_api = unsafe { host.context_api() }
    .ok_or_else(|| PluginResultC::err("context API unavailable"));
```

| 能力 | 查询方法 | 用途 |
|---|---|---|
| `CAPABILITY_CONTEXT` | `HostApiV1::context_api()` | 紧凑岛文本和状态资源 |
| `CAPABILITY_MEDIA` | `HostApiV1::media_api()` | 媒体 UI 数据和可选控制命令 |
| `CAPABILITY_I18N` | `HostApiV1::i18n_api()` | 翻译 bundle |
| `CAPABILITY_HOST_STATE` | `HostApiV1::host_state_api()` | 当前媒体和明暗主题快照 |

调用未在描述符中声明的能力会返回错误。

## Context 服务

Context 资源支持创建、更新和释放：

```rust
let data = ContextDataV1 {
    priority: PRIORITY_MEDIUM,
    flags: CONTEXT_FLAG_SHOW_COMPACT,
    timeout_ms: 5_000,
    title: str_to_fixed("Build complete"),
    body: str_to_fixed("Release package is ready"),
    compact_text: str_to_fixed("Build ready"),
    ..Default::default()
};

let mut id = INVALID_ID;
let result = unsafe { context_api.create.unwrap()(token, &data, &mut id) };
```

`timeout_ms = 0` 表示持续显示。超时只会隐藏 Context，资源所有权仍属于插件，插件仍应释放它。更新资源会刷新显示顺序和超时时间。

## Media 服务

活动 Media 资源会覆盖 SMTC。最近创建或更新的媒体资源会被显示；释放后会选择下一个插件媒体源，没有剩余资源时恢复 SMTC。

```rust
let cover = std::fs::read("cover.png").unwrap_or_default();
let media = MediaSourceDataV1 {
    flags: MEDIA_FLAG_PLAYING,
    title: str_to_fixed("Plugin Track"),
    artist: str_to_fixed("Plugin Artist"),
    duration_ms: 180_000,
    position_ms: 12_000,
    cover: ByteSliceV1::from_slice(&cover),
    ..Default::default()
};
```

宿主会在调用返回前复制封面，封面字节只需在本次调用期间有效。

需要媒体控制时，应声明相应的 `MEDIA_CONTROL_*` 标志并提供 `on_command`：

```rust
media.available_controls = MEDIA_CONTROL_TOGGLE_PLAY | MEDIA_CONTROL_SEEK;
media.on_command = Some(on_media_command);
media.callback_data = state_pointer;
```

回调在 WinIsland 事件循环线程同步执行，并允许再次调用宿主服务。`callback_data` 必须持续有效，直到媒体资源成功释放。回调执行期间，更新或释放同一资源会返回错误。

## 翻译服务

翻译字符串使用带长度的 UTF-8 借用切片，宿主会在注册调用期间复制：

```rust
let pairs = [TranslationPairV1 {
    key: Utf8SliceV1::borrowed("hello.title"),
    value: Utf8SliceV1::borrowed("Hello"),
}];
let mut bundle_id = INVALID_ID;
let result = unsafe {
    i18n_api.register_bundle.unwrap()(
        token,
        Utf8SliceV1::borrowed("en_us"),
        pairs.as_ptr(),
        pairs.len() as u32,
        &mut bundle_id,
    )
};
```

当前内置语言代码包括 `en_us`、`zh_cn` 和 `es_es`。插件应保留每个 bundle 的 `ResourceId`，并在 shutdown 中释放。

## Host State

调用 `get` 前使用 `HostStateV1::default()` 初始化，宿主会检查 `struct_size`：

```rust
let mut state = HostStateV1::default();
let result = unsafe { host_state_api.get.unwrap()(token, &mut state) };
if result.status == 0 {
    let playing = state.is_playing != 0;
}
```

快照包含 WinIsland 当前实际显示的媒体（包括插件媒体）以及 `light` 或 `dark` 主题文本。

## 资源所有权

- `PluginToken` 由 WinIsland 签发，插件不可自行构造或共享。
- `ResourceId` 由资源创建或注册接口返回。
- 更新和释放操作必须使用资源所属的 token。
- shutdown 成功后，WinIsland 会统一回收剩余资源。
- 插件仍应在 shutdown 中主动释放自己的资源。
- Context、Media 和翻译资源均有每插件数量及内存上限。
- 插件工作线程可以调用宿主服务，资源变更会主动唤醒 WinIsland 事件循环。

## FFI 规则

- 所有 ABI 结构体都使用 `#[repr(C)]`。
- 可版本扩展的结构体以 `struct_size` 开头；存在 `Default` 时应优先使用它初始化。
- 必需指针必须非空、满足对应类型对齐，并在整个调用期间可读或可写。
- `ByteSliceV1` 和 `Utf8SliceV1` 是 `(ptr, len)`，不是 NUL 结尾字符串。
- 除非字段明确要求更长生命周期，宿主都会在调用返回前复制借用数据。
- 不允许 panic 跨越 `extern "C"` 边界。
- 原生插件与 WinIsland 进程具有相同权限，属于受信任扩展。

## 打包和安装

加入 packager dev dependency：

```toml
[dev-dependencies]
winisland-plugin-api = { version = "0.3", features = ["packager"] }

[[example]]
name = "pack"
path = "package.rs"
```

```rust
fn main() {
    winisland_plugin_api::packager::PluginPackager::from_cargo()
        .unwrap()
        .build()
        .unwrap();
}
```

执行 `cargo run --example pack`。

`from_cargo()` 会读取包名、版本、作者、描述、repository 和 `[lib].name`。可以通过 builder 方法覆盖字段，但生成的 `plugin.yml` 仍必须与 `PluginMetadataC` 完全一致。

安装包只有一个入口 DLL：

```yaml
id: hello-winisland-plugin
name: hello-winisland-plugin
author: Example Author
version: 0.1.0
description: Minimal WinIsland ABI v1 plugin
github-link: https://github.com/example/hello-winisland-plugin
abi-version: 1
entry: hello_winisland_plugin.dll
```

把 ZIP 拖到 WinIsland 上即可安装。解压过程包含路径和大小限制，并先进入 staging 目录。更新时，WinIsland 会先验证新描述符，再停止旧插件，并通过目录备份实现失败回滚。安装包可以带额外依赖 DLL，但只有 `entry` 会作为插件加载。

Packager 可以写入 DLL 哈希和 Ed25519 签名；宿主目前尚未强制验证签名。
