# WinIsland — Milestone v1.1: 恢复上游合并后丢失的用户功能

## v1.1 Requirements

### Cover & Visual (封面与视觉)
- [ ] **COVER-01**: 封面支持方形/圆形切换，迷你态和展开态独立设置
- [ ] **COVER-02**: 圆形封面旋转动画（播放旋转，暂停停止，可开关）
- [ ] **COVER-03**: 非正方形封面居中裁剪，不拉伸变形
- [ ] **COVER-04**: 切歌封面翻转动画 + 旧封面保留直到新封面就绪
- [ ] **COVER-05**: 动态配色背景（封面主色调）+ 切歌时保留上一个有效颜色

### Lyrics (歌词)
- [ ] **LYRIC-01**: Widget 页面歌词显示，当前行居中突出
- [ ] **LYRIC-02**: 歌词切换平滑滚动动画（smoothstep）
- [ ] **LYRIC-03**: 长歌词自动来回滚动
- [ ] **LYRIC-04**: 暂停时冻结歌词，显示控制按钮

### Controls (交互控制)
- [ ] **CTRL-01**: 迷你态暂停时显示控制按钮
- [ ] **CTRL-02**: 迷你态始终可点击控制按钮

### Audio (音频)
- [ ] **AUDIO-01**: 音频门控联动（隐藏静音/显示开启）
- [ ] **AUDIO-02**: 启动时 SetMute 静音自身音频会话（已在 audio.rs 保留）

### Style & UI (风格与界面)
- [ ] **STYLE-01**: Mica 背景风格支持
- [ ] **STYLE-02**: 动态配色背景风格
- [ ] **UI-01**: 设置界面亮/暗主题跟随系统 + 手动切换
- [ ] **UI-02**: 设置界面子选项卡（外观/效果/行为）
- [ ] **UI-03**: 字体对比预览
- [ ] **UI-04**: 设置窗口单实例 + 可调整大小

### Fixes (修复)
- [ ] **FIX-01**: SMTC 切歌封面更新修复（已在 smtc.rs 保留）
- [ ] **FIX-02**: 暂停时停止持续重绘
- [ ] **FIX-03**: 自动隐藏行为修复

## Out of Scope

- 新功能（仅恢复已有）
- 插件系统扩展
- Dock 位置 UI 调整

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| COVER-01 ~ COVER-05 | Phase 1 | — |
| LYRIC-01 ~ LYRIC-04 | Phase 2 | — |
| CTRL-01 ~ CTRL-02 | Phase 1 | — |
| AUDIO-01 ~ AUDIO-02 | Phase 3 | — |
| STYLE-01 ~ STYLE-02 | Phase 1 | — |
| UI-01 ~ UI-04 | Phase 3 | — |
| FIX-01 | Phase 3 | — |
| FIX-02 ~ FIX-03 | Phase 3 | — |
