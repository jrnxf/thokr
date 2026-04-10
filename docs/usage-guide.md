# thokr 完整使用指南

## 目录

1. [简介](#简介)
2. [安装](#安装)
3. [快速入门](#快速入门)
4. [核心功能](#核心功能)
5. [高级用法](#高级用法)
6. [自定义配置](#自定义配置)
7. [故障排除](#故障排除)
8. [完整示例](#完整示例)
9. [相关工具](#相关工具)

---

## 简介

**thokr** 是一个优雅的终端打字练习工具，提供可视化结果和历史记录功能。

### 主要特性

- ⌨️ **打字练习** - 提升打字速度和准确度
- 📊 **可视化结果** - 实时显示 WPM、准确率等统计
- 📈 **历史记录** - 追踪进步趋势
- 🎨 **美观 TUI** - 现代化终端界面
- 🌍 **多语言支持** - English, English1K, English10K
- ⏱️ **灵活模式** - 时间模式、单词模式、句子模式

### 系统要求

- **平台**: Linux, macOS, Windows
- **终端**: 支持 UTF-8 和 Unicode
- **Rust**: 1.70+ (编译需要)

---

## 安装

### Cargo (推荐)

```bash
# 安装 Rust (如果未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 thokr
cargo install thokr

# 验证安装
thokr --version

# 升级
cargo install --force thokr

# 卸载
cargo uninstall thokr
```

### Docker

```bash
# 运行容器
docker run -it thatvegandev/thokr

# 或创建别名
alias thokr="docker run -it thatvegandev/thokr"
```

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/jrnxf/thokr
cd thokr

# 编译
cargo build --release

# 二进制位置
./target/release/thokr

# (可选) 安装到系统
sudo cp target/release/thokr /usr/local/bin/
```

**编译要求**:
- Rust 1.70+
- 1GB+ 可用内存
- 约 5 分钟编译时间

---

## 快速入门

### 启动 thokr

```bash
# 基本启动
thokr

# 显示帮助
thokr --help
```

### 界面导航

```
┌─────────────────────────────────────────────────────────┐
│                    thokr v0.4.1                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Words: 15    Time: 60s    Language: English           │
│                                                         │
│  the quick brown fox jumps over the lazy dog           │
│  ^^^                                                   │
│                                                         │
│  WPM: 0    Accuracy: 0%    Progress: 0%                │
│                                                         │
├─────────────────────────────────────────────────────────┤
│  Tab: New Test  R: Restart  Q: Quit                    │
└─────────────────────────────────────────────────────────┘
```

### 基本操作

| 按键 | 功能 |
|------|------|
| `Tab` | 开始新测试 |
| `R` | 重新开始 |
| `Q` | 退出程序 |
| `Backspace` | 删除字符 |
| `Enter` | 确认/继续 |

### 第一次练习

1. 启动 `thokr`
2. 按 `Tab` 开始新测试
3. 输入显示的文本
4. 完成后查看结果
5. 按 `R` 重新开始或 `Q` 退出

---

## 核心功能

### 测试模式

#### 1. 单词模式 (默认)

```bash
# 15 个单词测试
thokr -w 15

# 25 个单词测试
thokr -w 25

# 50 个单词测试
thokr -w 50
```

**适合**: 快速练习，日常训练

#### 2. 时间模式

```bash
# 30 秒测试
thokr -s 30

# 60 秒测试 (默认)
thokr -s 60

# 120 秒测试
thokr -s 120
```

**适合**: 速度训练，压力测试

#### 3. 句子模式

```bash
# 3 个句子
thokr -f 3

# 5 个句子
thokr -f 5

# 10 个句子
thokr -f 10
```

**适合**: 长文本练习，连贯性训练

#### 4. 自定义文本

```bash
# 使用自定义提示
thokr -p "The quick brown fox jumps over the lazy dog"

# 使用文件内容
cat myfile.txt | thokr -p "$(cat)"
```

**适合**: 特定文本练习，代码打字

### 语言选择

```bash
# English (默认词库)
thokr -l english

# English 1K (最常用 1000 词)
thokr -l english1k

# English 10K (常用 10000 词)
thokr -l english10k
```

**词库对比**:

| 词库 | 词汇量 | 难度 | 适合人群 |
|------|--------|------|---------|
| english1k | 1000 | 简单 | 初学者 |
| english | 5000 | 中等 | 进阶 |
| english10k | 10000 | 困难 | 高级 |

### 结果统计

测试完成后显示：

| 指标 | 说明 |
|------|------|
| **WPM** | 每分钟单词数 (Words Per Minute) |
| **Accuracy** | 准确率 (%) |
| **Progress** | 完成进度 (%) |
| **Time** | 用时 |
| **Errors** | 错误数 |

---

## 高级用法

### 组合选项

```bash
# 60 秒，25 个单词，高级词库
thokr -s 60 -w 25 -l english10k

# 自定义文本，时间限制 30 秒
thokr -p "custom text" -s 30

# 5 个句子，初级词库
thokr -f 5 -l english1k
```

### 管道输入

```bash
# 从文件读取
cat document.txt | thokr -p "$(cat)"

# 从命令输出
echo "Hello World" | thokr -p "$(cat)"
```

### 脚本化练习

```bash
#!/bin/bash
# daily-practice.sh

echo "Starting daily typing practice..."

# 热身：15 词
thokr -w 15

# 速度训练：30 秒
thokr -s 30

# 耐力训练：5 句子
thokr -f 5

# 高级挑战：10K 词库
thokr -l english10k -w 25

echo "Practice complete!"
```

### 结果记录

```bash
#!/bin/bash
# log-results.sh

LOG_FILE="typing-progress.log"
DATE=$(date '+%Y-%m-%d %H:%M:%S')

# 运行测试并捕获结果
thokr -w 15 > /tmp/result.txt 2>&1

# 解析结果 (简化示例)
WPM=$(grep "WPM" /tmp/result.txt | awk '{print $2}')
ACCURACY=$(grep "Accuracy" /tmp/result.txt | awk '{print $2}')

# 记录
echo "$DATE - WPM: $WPM, Accuracy: $ACCURACY" >> "$LOG_FILE"

echo "Results logged to $LOG_FILE"
```

---

## 自定义配置

### 环境变量

```bash
# 设置默认语言
export THOKR_LANGUAGE=english1k

# 设置默认单词数
export THOKR_WORDS=25

# 设置默认时间
export THOKR_TIME=60
```

### 配置文件

```toml
# ~/.config/thokr/config.toml

[general]
language = "english"
default_words = 15
default_time = 60
default_sentences = 3

[display]
show_wpm = true
show_accuracy = true
show_progress = true
theme = "default"

[history]
enabled = true
log_file = "~/.local/share/thokr/history.log"
max_entries = 1000
```

### 主题自定义

```toml
# ~/.config/thokr/themes/custom.toml
name = "Custom Theme"

[colors]
text = "#ffffff"
typed_correct = "#00ff00"
typed_wrong = "#ff0000"
cursor = "#ffff00"
background = "#1e1e1e"
stats = "#808080"
```

---

## 故障排除

### 常见问题

#### 1. "Command not found: thokr"

**原因**: 未安装或 PATH 未配置

**解决**:
```bash
# 验证安装
which thokr

# Cargo 安装后添加到 PATH
export PATH="$HOME/.cargo/bin:$PATH"

# 添加到 ~/.bashrc
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

#### 2. "Display issues" / 乱码

**原因**: 终端不支持 UTF-8

**解决**:
```bash
# 设置 UTF-8
export LANG=en_US.UTF-8
export LC_ALL=en_US.UTF-8

# 使用现代终端
alacritty -e thokr
# 或
kitty thokr
```

#### 3. 输入无响应

**原因**: 终端输入模式问题

**解决**:
```bash
# 重置终端
reset

# 或使用不同终端
# Windows Terminal, iTerm2, alacritty, kitty
```

#### 4. 性能缓慢

**原因**: 终端渲染慢

**解决**:
```bash
# 使用 GPU 加速终端
# alacritty, kitty, wezterm

# 减少窗口大小
# 降低终端分辨率
```

#### 5. 历史记录丢失

**原因**: 配置文件权限问题

**解决**:
```bash
# 检查目录权限
ls -la ~/.local/share/thokr/

# 修复权限
chmod 755 ~/.local/share/thokr/
```

### 性能优化

#### 提高响应速度

```bash
# 使用高性能终端
alacritty thokr
# 或
kitty thokr

# 避免在慢速 SSH 连接使用
```

#### 减少资源使用

```bash
# 禁用历史记录 (如果不需要)
# ~/.config/thokr/config.toml
[history]
enabled = false
```

---

## 完整示例

### 示例 1: 日常练习

```bash
#!/bin/bash
# daily-routine.sh

echo "=== Daily Typing Practice ==="
echo ""

# 热身
echo "1. Warm-up (15 words)"
thokr -w 15

# 速度
echo "2. Speed Test (30 seconds)"
thokr -s 30

# 准确度
echo "3. Accuracy Test (50 words)"
thokr -w 50

# 耐力
echo "4. Endurance (5 sentences)"
thokr -f 5

echo ""
echo "=== Practice Complete ==="
```

### 示例 2: WPM 挑战

```bash
#!/bin/bash
# wpm-challenge.sh

TARGET_WPM=60
echo "Target WPM: $TARGET_WPM"
echo ""

for i in {1..5}; do
    echo "Attempt $i:"
    thokr -s 60
    echo ""
    echo "Press Enter for next attempt..."
    read
done

echo "Challenge complete!"
```

### 示例 3: 代码打字练习

```bash
#!/bin/bash
# code-typing.sh

CODE_SNIPPETS=(
    "fn main() { println!(\"Hello, world!\"); }"
    "const fn = () => { return true; }"
    "class Person { constructor(name) { this.name = name; } }"
    "SELECT * FROM users WHERE active = true;"
)

for snippet in "${CODE_SNIPPETS[@]}"; do
    echo "Type the following code:"
    echo "$snippet"
    echo ""
    thokr -p "$snippet"
    echo ""
    echo "Press Enter to continue..."
    read
done
```

### 示例 4: 进度追踪

```bash
#!/bin/bash
# track-progress.sh

LOG_FILE="typing-log.csv"
DATE=$(date '+%Y-%m-%d')

# 如果文件不存在，创建表头
if [ ! -f "$LOG_FILE" ]; then
    echo "Date,WPM,Accuracy,Mode" > "$LOG_FILE"
fi

# 运行测试
echo "Starting test..."
RESULT=$(thokr -w 15 2>&1)

# 解析结果 (简化)
WPM=$(echo "$RESULT" | grep "WPM" | awk '{print $2}')
ACCURACY=$(echo "$RESULT" | grep "Accuracy" | awk '{print $2}')

# 记录
echo "$DATE,$WPM,$ACCURACY,words-15" >> "$LOG_FILE"

echo "Progress logged!"
echo "View progress: cat $LOG_FILE"
```

### 示例 5: 多人竞赛

```bash
#!/bin/bash
# typing-competition.sh

PLAYERS=("Alice" "Bob" "Charlie")
SCORES=()

echo "=== Typing Competition ==="
echo ""

for player in "${PLAYERS[@]}"; do
    echo "$player's turn!"
    echo "Press Enter to start..."
    read
    
    thokr -s 60
    
    echo ""
    echo "Press Enter for next player..."
    read
done

echo "=== Competition Complete ==="
```

---

## 相关工具

### 打字练习替代

| 工具 | 类型 | 说明 |
|------|------|------|
| **thokr** | TUI | 本工具 |
| **type-rs** | TUI | 另一个 Rust 打字工具 |
| **ttyper** | TUI | 终端打字练习 |
| **keybr.com** | Web | 在线打字练习 |
| **monkeytype** | Web | 流行在线平台 |

### 终端工具

| 工具 | 用途 |
|------|------|
| **alacritty** | GPU 加速终端 |
| **kitty** | 功能丰富终端 |
| **wezterm** | 跨平台终端 |
| **tmux** | 终端复用器 |

### 学习资源

- **TypingClub**: https://www.typingclub.com/
- **Keybr**: https://www.keybr.com/
- **Monkeytype**: https://monkeytype.com/

---

## 社区

- **Repository**: https://github.com/jrnxf/thokr
- **Issues**: https://github.com/jrnxf/thokr/issues
- **Discussions**: https://github.com/jrnxf/thokr/discussions

---

*Community contribution - unofficial comprehensive guide*  
*Last updated: 2026-04-10*  
*Word count: ~10KB*
