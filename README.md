<p align="center">
  <img src="resources/info-en.png" width="350" alt="">
</p>

## WinIsland

English | [简体中文](./README-zh.md)
This is a project that can display **Dynamic Island** on Windows.

## Download it
you can in [Release](https://github.com/Eatgrapes/WinIsland/releases) or our [website](https://tanikaze.icu/WinIsland) Download WinIsland

## Build it 

- **Rust** environment
- **Cargo**

```cmd
git clone https://github.com/Eatgrapes/WinIsland.git

cd WinIsland

cargo build --release
```

## Smtc (Only Chinese)
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

## Contributes 
We encourage contributions

If you have the energy or interest, PR are welcome

And You should see [Contributing](CONTRIBUTING.md)

> [!IMPORTANT]
> Any PRs not following the [Contributing Guidelines](CONTRIBUTING.md) will be closed.

## LICENCE
This project is subject to the [GNU General Public License v3.0](LICENSE).
