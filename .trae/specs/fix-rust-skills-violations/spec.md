# Rust Skills 全局代码质量修复 Spec

## Why
根据 rust-skills 179 条规则对代码库进行全面审查，发现 24 个 CRITICAL/HIGH 级别问题，主要集中在生产代码中的 `.unwrap()` 滥用（可能导致不可恢复的 panic）和 `unsafe` 块缺少 SAFETY 注释。这些问题在极端情况下会导致应用崩溃且无法恢复。

## What Changes
- 替换 main.rs 中的 3 个 `.unwrap()` 为安全的错误处理
- 替换 app.rs 中的 8 个 `.unwrap()` 为安全的错误处理（含窗口创建、Surface 初始化、current_exe 等）
- 替换 audio.rs 中的 Mutex `.unwrap()` 为 poison-safe 处理
- 替换 font.rs 中的 5 个 `.unwrap()` 为安全的回退逻辑
- 修复 audio.rs 中 FFT 处理结果被忽略的问题
- 为所有 `unsafe` 块添加 `// SAFETY:` 注释（10 处）

## Impact
- Affected code: src/main.rs, src/window/app.rs, src/core/audio.rs, src/utils/font.rs, src/utils/backdrop.rs
- 行为变更：部分错误场景从 panic 变为优雅降级（如窗口创建失败时退出而非崩溃）
- 无破坏性变更

## ADDED Requirements

### Requirement: 生产代码中禁止裸 .unwrap()
系统 SHALL NOT 在可恢复错误路径上使用 `.unwrap()`。所有 `.unwrap()` 应替换为：
- `match` + 错误日志 + 优雅降级（启动路径）
- `.unwrap_or_default()` / `.unwrap_or(fallback)`（热路径）
- `.map(|g| *g).unwrap_or(default)`（Mutex poison 安全）

#### Scenario: 窗口创建失败
- **WHEN** `create_window` 返回 Err
- **THEN** 记录错误日志并安全退出，而非 panic

#### Scenario: Mutex 被 poison
- **WHEN** 音频线程 panic 导致 Mutex poison
- **THEN** `get_spectrum()` 返回静音频谱 `[0.0; 6]` 而非 panic

### Requirement: unsafe 块必须包含 SAFETY 注释
所有 `unsafe` 块 SHALL 在其上方包含 `// SAFETY:` 注释，说明为何该 unsafe 调用是安全的。

#### Scenario: 审查 unsafe 代码
- **WHEN** 开发者阅读任何 unsafe 块
- **THEN** 可以通过 SAFETY 注释理解安全保证的依据

### Requirement: FFT 处理错误应清零输出
系统 SHALL 在 FFT 处理失败时将输出缓冲区清零，而非使用可能未初始化的数据。

#### Scenario: FFT 处理失败
- **WHEN** `fft.process()` 返回 Err
- **THEN** 输出缓冲区被清零并记录警告日志
