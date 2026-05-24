# 代码质量修复计划 v2

## 审查来源
基于 rust-skills 规则对最近修改的 5 个文件进行审查，结合用户要求增加初次加载音乐进度识别修复。

## 修复任务

### Task 1: CRITICAL - smtc.rs 热路径磁盘 I/O 风暴 (C-01)
- **文件**: src/core/smtc.rs L384-424
- **问题**: `auto_allow_new_apps` 在每次 `update_media_info` 调用时执行 `load_config()` + `save_config()`，约每 300ms 一次磁盘读写
- **修复**: 将配置加载/保存移出热路径，仅在检测到新应用时才加载和保存
- **优先级**: CRITICAL

### Task 2: CRITICAL - app.rs current_exe().unwrap() 可能 panic (C-02)
- **文件**: src/window/app.rs L498, L865, L878
- **问题**: `std::env::current_exe().unwrap()` 在极端情况下可能返回 Err
- **修复**: 使用 `if let Ok(exe) = std::env::current_exe()` 并在 Err 时优雅降级
- **优先级**: CRITICAL

### Task 3: CRITICAL - NonZeroU32::new().unwrap() 零值 panic (C-03)
- **文件**: src/window/app.rs L681-683, L937-938
- **问题**: `os_w`/`os_h` 为 0 时 `NonZeroU32::new(0)` 返回 None，`.unwrap()` panic
- **修复**: 计算后添加 `.max(1)` 下界保护
- **优先级**: CRITICAL

### Task 4: HIGH - 所有 unsafe 块缺少 SAFETY 注释 (H-01)
- **文件**: app.rs (5处), settings.rs (4处), audio.rs (4处)
- **修复**: 为每个 unsafe 块添加 `// SAFETY:` 注释
- **优先级**: HIGH
- **注意**: 仅添加注释，不修改逻辑

### Task 5: HIGH - settings.rs window.as_ref().unwrap() (H-02)
- **文件**: src/window/settings.rs L794, L1263, L1313, L1674, L1839, L1985, L2033
- **问题**: `self.window.as_ref().unwrap()` 在窗口未创建时 panic
- **修复**: 使用 `if let Some(win) = &self.window` 模式或 early return
- **优先级**: HIGH

### Task 6: HIGH - audio.rs Mutex::lock().unwrap() poison 风险 (H-05)
- **文件**: src/core/audio.rs L42
- **问题**: `spectrum.lock().unwrap()` 在 Mutex poison 时 panic
- **修复**: 使用 `.lock().unwrap_or_else(|e| e.into_inner())`
- **优先级**: HIGH

### Task 7: HIGH - settings.rs OpenMutexW is_err+unwrap 逻辑混乱 (H-03)
- **文件**: src/window/settings.rs L2167
- **问题**: `if h.is_err() { ... } h.unwrap()` 模式不安全
- **修复**: 改为 `if let Ok(handle) = h` 模式
- **优先级**: HIGH

### Task 8: MEDIUM - audio.rs FFT 处理结果被静默忽略 (M-08)
- **文件**: src/core/audio.rs L226
- **问题**: `let _ = fft.process(...)` 忽略错误
- **修复**: 添加错误日志 + 输出清零
- **优先级**: MEDIUM

### Task 9: MEDIUM - settings.rs partial_cmp().unwrap() NaN panic (M-04)
- **文件**: src/window/settings.rs L2072
- **问题**: `partial_cmp().unwrap()` 在 NaN 时 panic
- **修复**: 使用 `total_cmp` 替代
- **优先级**: MEDIUM

### Task 10: MEDIUM - audio.rs AtomicU32 存储 f32 缺少文档 (M-05)
- **文件**: src/core/audio.rs L27-28
- **修复**: 添加注释说明为何使用 AtomicU32 存储位模式
- **优先级**: MEDIUM
- **注意**: 仅添加注释

### Task 11: MEDIUM - smtc.rs position_ms 溢出风险 (C-04)
- **文件**: src/core/smtc.rs L69
- **问题**: `position_ms + elapsed` 可能溢出 u64
- **修复**: 使用 `saturating_add`
- **优先级**: MEDIUM

### Task 12: 初次加载音乐进度识别修复
- **文件**: src/core/smtc.rs L225-243
- **问题**: 初始更新重试逻辑仅在 `position_ms > 0` 时认为成功，但如果音乐刚暂停在 0ms 位置也会误判为失败；另外重试间隔 100ms 可能不够 SMTC 就绪
- **修复**: 改进初始进度获取逻辑，增加对 duration 的检查作为就绪标志
- **优先级**: HIGH

### Task 13: LOW - eprintln! 改为 log 宏 (L-05, L-06)
- **文件**: settings.rs L1460, L1706; audio.rs L211
- **修复**: `eprintln!` → `log::warn!` / `log::error!`
- **优先级**: LOW

### Task 14: 编译验证
- `cargo check` 无错误
- `cargo clippy` 无新警告

## 不修复项（添加注释说明原因）

### N-01: config.rs FromStr 错误类型为 () (M-01)
- **原因**: DockPosition 仅在内部配置解析使用，错误信息对用户无意义，改动收益低

### N-02: config.rs Default 与 serde default 函数值重复 (M-02)
- **原因**: 当前模式是 serde 反序列化的标准做法，重构为常量引用会增加复杂度

### N-03: config.rs font_size 默认值 0.0 语义不清 (M-03)
- **原因**: 0.0 在渲染代码中已有特殊处理（使用系统默认），属于项目内部约定

### N-04: smtc.rs size as u32 截断 (M-06)
- **原因**: 缩略图大小不会超过 u32 范围，实际场景中不存在截断风险

### N-05: settings.rs 重置默认配置丢失 smtc_known_apps (M-07)
- **原因**: 重置后 auto_allow_new_apps 会自动重新发现，属于预期行为

### N-06: app.rs 硬编码魔法数字 (L-01, L-02)
- **原因**: 提取常量属于代码风格优化，不影响功能正确性

### N-07: app.rs thread::sleep 帧率限制 (L-03)
- **原因**: 当前方案虽不精确但可工作，改为 ControlFlow 需要较大重构

### N-08: smtc.rs 初始化失败静默退出 (L-04)
- **原因**: SMTC 不可用时无媒体信息是合理的默认行为，添加日志即可（已在 Task 13 覆盖）

### N-09: config.rs 部分 AppConfig 字段缺 serde(default) (L-07)
- **原因**: 已有字段都有 default 函数，新字段已添加；旧字段改动需评估兼容性影响

## Task 依赖
- Task 14 依赖 Task 1-13
- Task 1-13 相互独立，可并行
- Task 4, 10, N-01~N-09 仅添加注释，风险最低
