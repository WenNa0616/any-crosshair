# Any Crosshair

使用任意 PNG 作为屏幕准心的 Windows 桌面工具。

> 基于 [qinghon/any-crosshair](https://github.com/qinghon/any-crosshair) 修改而来，感谢原作者。

## 功能

- 任意 PNG 图片作为屏幕准心，透明叠加在屏幕中央
- 系统托盘图标：左键单击切换显示/隐藏，右键打开菜单
- 热键切换显示/隐藏（支持 F9~F12、Ctrl+F9~F12）
- 开机自启（通过注册表）
- 默认隐藏，按热键或单击托盘显示
- 单文件运行，无需额外配置文件

## 使用方法

1. 从 [Releases](../../releases) 下载 `any-crosshair.exe`
2. 双击运行，准心默认隐藏
3. 按 **F9**（默认热键）或单击托盘图标显示/隐藏准心
4. 右键托盘图标可切换热键、开机自启或退出

### 自定义准心

在 `any-crosshair.exe` 同目录放置 `default.png` 文件，程序会优先使用该图片作为准心。

```
any-crosshair.exe    # 主程序（内嵌默认准心）
default.png          # 可选，自定义准心图片
```

## 热键

| 热键 | 说明 |
|------|------|
| F9 | 显示/隐藏准心（默认） |
| F10~F12 | 可选 |
| Ctrl+F9~F12 | 可选 |

通过右键托盘菜单 -> 热键 可切换。

## 准心制作

- 在线制作：https://kovaaks.com/kovaaks/crosshair-creator
- 自定义 PNG：只要能制作透明 PNG 均可

## 编译

```bash
cargo build --release
```

输出：`target/release/any-crosshair.exe`

## 致谢

- [qinghon/any-crosshair](https://github.com/qinghon/any-crosshair) - 原项目
