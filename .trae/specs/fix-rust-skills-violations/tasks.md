# Tasks

- [ ] Task 1: 修复 main.rs 中的 3 个 .unwrap()
  - [ ] 1.1: `Runtime::new().unwrap()` → match + 错误日志
  - [ ] 1.2: `EventLoop::new().unwrap()` → match + 错误日志
  - [ ] 1.3: `run_app().unwrap()` → if let Err + 错误日志
  - [ ] 1.4: 为 2 个 unsafe 块添加 SAFETY 注释

- [ ] Task 2: 修复 app.rs 中的 8 个 .unwrap()
  - [ ] 2.1: `create_window().unwrap()` (L628) → match + return
  - [ ] 2.2: `Context::new().unwrap()` + `Surface::new().unwrap()` + 2x `NonZeroU32::new().unwrap()` (L677-684) → match + return
  - [ ] 2.3: `NonZeroU32::new().unwrap()` x2 (L935-936) → if let Some
  - [ ] 2.4: `current_exe().unwrap()` x3 (L498, L863, L876) → if let Ok
  - [ ] 2.5: 为 5 个 unsafe 块添加 SAFETY 注释

- [ ] Task 3: 修复 audio.rs 中的 Mutex 和 FFT 问题
  - [ ] 3.1: `spectrum.lock().unwrap()` (L42) → `.map(|g| *g).unwrap_or([0.0; 6])`
  - [ ] 3.2: `fft.process()` 结果被忽略 (L226) → if let Err + 清零输出
  - [ ] 3.3: 为 3 个 unsafe 块添加 SAFETY 注释

- [ ] Task 4: 修复 font.rs 中的 5 个 .unwrap()
  - [ ] 4.1: `legacy_make_typeface(None, style).unwrap()` x3 (L115, L139, L184) → unwrap_or_else + 回退
  - [ ] 4.2: `cache_mut.get(&cache_key).unwrap()` x2 (L261, L303) → if let Some

- [ ] Task 5: 修复 backdrop.rs 中的 SAFETY 注释
  - [ ] 5.1: 为 try_enable_mica 的 unsafe 块添加 SAFETY 注释

- [ ] Task 6: 编译验证
  - [ ] 6.1: `cargo check` 无错误
  - [ ] 6.2: `cargo clippy` 无新警告

# Task Dependencies
- Task 6 depends on Task 1-5
- Task 1-5 are independent and can be parallelized
