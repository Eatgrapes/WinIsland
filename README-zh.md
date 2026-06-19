<p align="center">
  <img src="resources/info-zh.png" width="350">
</p>

# WinIsland

[English](./README.md) | 简体中文

这是一个可以在 Windows 上显示**灵动岛**的项目。

## 下载
你可以在 [Release](https://github.com/Eatgrapes/WinIsland/releases) 或者我们的 [网站](https://tanikaze.icu/WinIsland) 下载WinIsland

## 构建项目

- **Rust** 环境
- **Cargo**

```cmd
git clone https://github.com/Eatgrapes/WinIsland.git

cd WinIsland

cargo build --release
```

## Smtc
> [!WARNING]
> 网易云，酷狗等国产音乐软件的smtc均无法读取正常的数据，与此相关的issue将会标记为重复。

网易云修复方案：
1. 确保网易云的设置的smtc选项关闭
2. 下载并安装[BetterNCM](https://github.com/std-microblock/chromatic/releases/tag/1.3.4) (或者 [chromatic](https://github.com/std-microblock/chromatic) 不确定是否能用)
3. 安装[Inflink-rs](https://github.com/apoint123/inflink-rs)插件
4. 在BetterNCM的设置找到Inflink，打开smtc选项
5. 体验你的WinIsland！😋

其他音乐软件暂无替代方案
等待更新适配

## 贡献
我们欢迎任何形式的贡献！

如果你有精力或兴趣，欢迎提交 PR。

在提PR前，请先看[Contributing](./Docs/CONTRIBUTING-zh.md)

> [!IMPORTANT]
> 所有未遵守[贡献指南](CONTRIBUTING.md)的PR将会被close

## 许可证
本项目遵循 [GNU General Public License v3.0.](LICENSE)
