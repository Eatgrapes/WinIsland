# WinIsland v1.1 — Roadmap

**Milestone**: 恢复上游合并后丢失的用户功能  
**Baseline**: baba305 (Merge upstream/master)  
**Strategy**: 按文件依赖关系分 3 个 Phase，每个 Phase 内并行恢复

---

### Phase 1: Core Render — 封面与渲染

**Goal**: 恢复 render.rs 和 music_view.rs 中的封面形状/旋转/动态配色功能

**Mode:** mvp

**Files**: `src/core/render.rs`, `src/ui/expanded/music_view.rs`

**Requirements**: COVER-01, COVER-02, COVER-03, COVER-04, COVER-05, STYLE-01, STYLE-02, CTRL-01

**Success Criteria**:
1. 封面可切换方形/圆形，迷你态和展开态独立生效
2. 圆形封面播放时旋转、暂停时停止
3. 非正方形封面居中裁剪不变形
4. 切歌时封面翻转动画流畅
5. 动态配色背景从封面提取主色调，切歌不闪烁
6. Mica 和动态配色风格可选用
7. 迷你态暂停时显示控制按钮

---

### Phase 2: Lyrics Widget — 歌词显示

**Goal**: 恢复 widget_view.rs 中的歌词显示和滚动动画

**Mode:** mvp

**Files**: `src/ui/expanded/widget_view.rs`

**Requirements**: LYRIC-01, LYRIC-02, LYRIC-03, LYRIC-04

**Success Criteria**:
1. Widget 页面显示歌词，当前行居中突出
2. 歌词切换有平滑滚动动画
3. 长歌词自动来回滚动，两端停顿
4. 暂停时冻结歌词，隐藏歌词显示控制按钮

---

### Phase 3: App Logic & Settings — 交互与设置界面

**Goal**: 恢复 app.rs 的音频门控/自动隐藏逻辑，以及 settings.rs + renderer.rs 的设置界面功能

**Mode:** mvp

**Files**: `src/window/app.rs`, `src/window/settings.rs`, `src/utils/settings_ui/renderer.rs`

**Requirements**: AUDIO-01, AUDIO-02, CTRL-01, UI-01, UI-02, UI-03, UI-04, FIX-01, FIX-02, FIX-03

**Success Criteria**:
1. 音频门控联动：隐藏时静音，显示时开启（可开关）
2. 设置界面跟随系统亮/暗主题，可手动切换
3. 设置界面子选项卡（外观/效果/行为）正常切换
4. 字体对比预览正常显示
5. 设置窗口单实例，可调整大小
6. 暂停时停止持续重绘（CPU 优化）
7. 自动隐藏行为正确（暂停时允许自动隐藏）
