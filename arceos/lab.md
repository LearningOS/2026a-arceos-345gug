# ArceOS 六个练习实验报告

## 1. 实验概述

本实验在 ArceOS（一个基于 Rust 的 unikernel 操作系统框架）上依次完成六个练习，覆盖控制台 I/O（Input/Output，输入/输出）、RAMFS（Random Access Memory File System，内存文件系统）、内存分配器、哈希容器、系统调用与文件映射、RISC-V 虚拟化六个方向。

六个练习均位于 `exercises/` 目录下，采用 `no_std`（不链接 Rust 标准库）与 `no_main`（不生成标准 `main` 启动胶水代码）方式构建，运行于 QEMU（Quick Emulator）模拟的 RISC-V 64 位 `virt` 平台。

| 序号 | 练习 | 对应操作系统模块 | 核心机制 | 测试脚本 | 结果 |
| --- | --- | --- | --- | --- | --- |
| 1 | `print_with_color` | I/O | ANSI（American National Standards Institute，美国国家标准学会）转义序列 | `./scripts/test-print.sh` | PASS |
| 2 | `ramfs_rename` | 文件系统 | VFS（Virtual File System，虚拟文件系统）同目录重命名 | `./scripts/test-ramfs_rename.sh` | PASS |
| 3 | `alt_alloc` | 内存分配 | 双端 bump（指针递增）分配器 | `./scripts/test-alt_alloc.sh` | PASS |
| 4 | `support_hashmap` | 容器 | `hashbrown` 哈希表与 `no_std` 默认哈希器 | `./scripts/test-support_hashmap.sh` | PASS |
| 5 | `sys_map` | 系统调用 | `mmap`（memory map，内存映射）文件映射 | `./scripts/test-sys_map.sh` | PASS |
| 6 | `simple_hv` | 虚拟化 | RISC-V H（Hypervisor，虚拟化）扩展与两阶段地址翻译 | `./scripts/test-simple_hv.sh` | PASS |

全部测试的最终检查结果均为 `ALL CHECKS PASSED`，QEMU 进程正常退出（退出码 0）。课程提供的统一评分脚本 `../scripts/total-test.sh` 的最后一行为 `600`（六个练习各 100 分）。

### 1.1 交付目录

```text
exercises/
├── print_with_color/       # 练习 1
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── scripts/test-print.sh
├── ramfs_rename/           # 练习 2
│   ├── Cargo.toml
│   ├── src/main.rs
│   ├── src/ramfs.rs
│   └── scripts/test-ramfs_rename.sh
├── alt_alloc/              # 练习 3
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── scripts/test-alt_alloc.sh
├── support_hashmap/        # 练习 4
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── scripts/test-support_hashmap.sh
├── sys_map/                # 练习 5
│   ├── Cargo.toml
│   ├── src/{main,syscall,loader,task}.rs
│   └── scripts/test-sys_map.sh
├── simple_hv/              # 练习 6
│   ├── Cargo.toml
│   ├── src/{main,loader,vcpu,regs,csrs,guest.S}.rs
│   ├── src/sbi/            # SBI 解码子模块
│   └── scripts/test-simple_hv.sh
├── scripts/                # 六个练习的统一入口脚本
│   ├── test-print.sh
│   ├── test-ramfs_rename.sh
│   ├── test-alt_alloc.sh
│   ├── test-support_hashmap.sh
│   ├── test-sys_map.sh
│   └── test-simple_hv.sh
└── tools/mkdisk.py         # 无 mtools 依赖的 FAT32 镜像生成器
```

## 2. 实验环境与平台

### 2.1 硬件平台与模拟器

- 模拟器：QEMU 10.2.1 的 `qemu-system-riscv64`，机器类型 `virt`。
- 架构：RISC-V（Reduced Instruction Set Computer – Five，第五代精简指令集计算机）64 位（RV64GC），使用 Sv39 分页。
- 固件：OpenSBI（Open Supervisor Binary Interface）v1.8，作为 M-mode（Machine mode，机器模式）固件提供 SBI 服务。
- 内存：128 MiB，单 hart（硬件线程）。
- 设备：`virtio-blk-pci`（PCI，Peripheral Component Interconnect，外设组件互连）块设备用于挂载 FAT（File Allocation Table，文件分配表）文件系统；串口 8250 作为控制台。
- 虚拟机监视器 CPU（Central Processing Unit，中央处理器）配置：普通练习使用 QEMU 默认 CPU；虚拟化练习显式使用 `QEMU_CPU=max`，保证 H 扩展存在。

### 2.2 软件平台

- 操作系统框架：ArceOS unikernel，全部内核态代码运行在同一地址空间，无用户态/内核态隔离的传统宏内核结构。
- 构建系统：仓库顶层 `Makefile`，通过 `A=exercises/<name>` 指定应用，`make run` 自动完成编译、链接、`rust-objcopy` 转换与 QEMU 启动。
- 目标三元组：`riscv64gc-unknown-none-elf`。
- Rust 工具链：`nightly-2024-09-04`，`rustc 1.83.0-nightly`，启用 `panic=abort`、`-C relocation-model=static`，内核链接到 `0xffff_ffc0_8020_0000`。
- 用户库：`axstd`，提供类似 Rust `std` 的 `println!`、`Vec`、`File`、`HashMap` 等接口；底层为 `axfeat`、`arceos_api`、`axhal`、`axfs`、`axalloc`、`axtask`、`axmm` 等模块。
- 内存分配：默认使用 `axalloc` 的 TLSF（Two-Level Segregated Fit，两级隔离适配）字节分配器加位图页分配器；练习 3 替换为 `alt_axalloc` 的双端 bump 分配器。
- 测试驱动：每个练习提供独立 shell 脚本，脚本负责编译、启动 QEMU、捕获串口输出、用 `grep` 校验关键行，并在失败时返回非零退出码。

### 2.3 术语约定

- **ANSI**：American National Standards Institute，美国国家标准学会；本文指其终端控制转义序列标准。
- **RAMFS**：Random Access Memory File System，内存文件系统。
- **VFS**：Virtual File System，虚拟文件系统接口层。
- **SBI**：Supervisor Binary Interface，管理者二进制接口，S-mode 与 M-mode 固件之间的调用约定。
- **CSR**：Control and Status Register，控制与状态寄存器。
- **GPA / HPA**：Guest Physical Address（客户机物理地址）/ Host Physical Address（宿主机物理地址）。
- **MMU**：Memory Management Unit，内存管理单元。
- **TLB**：Translation Lookaside Buffer，地址翻译快表。
- **ELF**：Executable and Linkable Format，可执行与可链接格式。
- **FAT**：File Allocation Table，文件分配表。
- **BPB**：BIOS（Basic Input/Output System，基本输入输出系统）Parameter Block，BIOS 参数块，FAT 引导扇区中的元数据。
- **HS-mode / VS-mode / M-mode / S-mode / U-mode**：Hypervisor-extended Supervisor mode（扩展管理程序的管理者模式）/ Virtual Supervisor mode（虚拟管理者模式）/ Machine mode（机器模式）/ Supervisor mode（管理者模式）/ User mode（用户模式）。
- **`hgatp`**：Hypervisor Guest Address Translation and Protection，客户机地址翻译与保护寄存器。
- **MMU 两阶段翻译**：VS-mode 下先由 `vsatp` 完成客户机虚拟地址到 GPA 的翻译，再由 `hgatp` 完成 GPA 到 HPA 的翻译。
- **`ecall`**：Environment Call，环境调用指令，用于发起系统调用或 SBI 调用。
- **`sret`**：Supervisor Return，从 S-mode/HS-mode 返回到较低特权级的指令。

### 2.4 通用编译与运行模型

所有练习均通过 `Makefile` 驱动，核心形式为：

```bash
cd /path/to/arceos
make A=exercises/<exercise> [FEATURES=...] [BLK=y DISK_IMG=...] [QEMU_CPU=...] \
     LOG=<level> QEMU_LOG=n PFLASH=n run
```

其中：

- `A=` 指定应用 crate；
- `FEATURES=` 追加 `axstd`/`axfeat` 特性，例如 `driver-ramdisk`；
- `BLK=y` 挂载 `virtio-blk` 块设备，`DISK_IMG=` 指定 FAT32 镜像；
- `QEMU_CPU=` 指定 QEMU CPU 型号/特性；
- `LOG=` 控制 `axlog` 日志级别；
- `QEMU_LOG=n` 关闭 QEMU 指令级日志，避免生成巨大的 `qemu.log`；
- `PFLASH=n` 关闭不必要的 pflash 设备。

各应用结束时会调用 `axhal::misc::terminate()`（内部通过 SBI SRST，System Reset，系统复位扩展关机），使 QEMU 自动退出，测试脚本无需交互式终止。

课程评分脚本位于仓库上一级的 `scripts/total-test.sh`，依次调用六个官方测试。其中 `make disk_img`/`update_disk.sh` 依赖 GNU mtools（`mcopy`、`mmd`）；本环境已把 mtools 安装到 `~/.local/bin`（该目录在 `PATH` 中）。官方 `make payload` 还需要 `riscv64-linux-musl-gcc`，本环境没有该交叉编译器，因此 `payload/mapfile_c/Makefile` 会自动回退到 Rust 版 `mapfile`，`hello_c`/`fileops_c` 在没有编译器时跳过；在有 musl 工具的 Docker 环境中仍使用原始 C 程序。

---

## 3. 练习一：`print_with_color`

### 3.1 实验目的

在 ArceOS 的串口控制台上输出带颜色的问候语，掌握：

1. 终端 ANSI 转义序列的组成与语义；
2. `axstd::println!` 在 `no_std` 环境下的工作方式；
3. QEMU `-nographic` 串口透传与终端着色之间的关系；
4. 如何用测试脚本对包含 ESC（Escape，转义）控制字符的输出做字节级校验。

### 3.2 原理分析

ANSI 颜色控制属于 SGR（Select Graphic Rendition，选择图形再现）序列，格式为：

```text
ESC [ <参数> m
```

其中 `ESC` 的字节值为 `0x1b`（C 语言写作 `\x1b`，Rust 字符串写作 `\x1b` 或 `\u{1b}`）。常用参数：

| 参数 | 含义 |
| --- | --- |
| `0` | 重置所有属性（颜色、加粗、下划线等） |
| `1` | 加粗/高亮 |
| `31` | 红色前景 |
| `32` | 绿色前景 |
| `33` | 黄色前景 |
| `34` | 蓝色前景 |
| `35` | 品红前景 |
| `36` | 青色前景 |

序列由终端解释，而不是由 QEMU 解释。QEMU 的 `-nographic` 模式把串口字节原样转发到宿主机的标准输出，因此控制台日志文件里会保留原始 ESC 字节；支持 ANSI 的终端会显示颜色，`grep`/`cat -v` 则能看到 `^[[32m` 之类的文本。

ArceOS 的 `println!` 最终调用 `axstd::io::stdout()`，在 `axhal::console` 中逐字节写入 8250 UART（Universal Asynchronous Receiver/Transmitter，通用异步收发器）。转义序列只是普通字节，不需要内核提供特殊支持。

### 3.3 核心代码（逐行注释）

文件：`exercises/print_with_color/src/main.rs`

```rust
//! Exercise 1: colored console output.
//!
//! ANSI（American National Standards Institute）escape sequences are used to
//! switch the foreground color of the serial console.  A sequence has the form
//! `ESC [ <SGR parameters> m`, where `ESC` is the byte `0x1b` and SGR means
//! Select Graphic Rendition.  `ESC[0m` resets all attributes.

#![cfg_attr(feature = "axstd", no_std)]   // 有 axstd 时不链接标准库
#![cfg_attr(feature = "axstd", no_main)]  // 不生成标准 main 启动代码

#[cfg(feature = "axstd")]
use axstd::println;                        // ArceOS 提供的类 std 打印宏

/// Color related ANSI escape sequences.
mod ansi {
    pub const RESET: &str = "\x1b[0m";    // 关闭颜色/加粗等属性
    pub const BOLD: &str = "\x1b[1m";     // 加粗
    pub const RED: &str = "\x1b[31m";     // 红色前景
    pub const GREEN: &str = "\x1b[32m";   // 绿色前景
    pub const YELLOW: &str = "\x1b[33m";  // 黄色前景
    pub const BLUE: &str = "\x1b[34m";    // 蓝色前景
    pub const MAGENTA: &str = "\x1b[35m"; // 品红前景
    pub const CYAN: &str = "\x1b[36m";    // 青色前景
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 问候语：绿色加粗的标签 + 青色加粗的正文
    println!(
        "{}{}[WithColor]{}: {}{}Hello, Arceos!{}",
        ansi::BOLD, ansi::GREEN, ansi::RESET,
        ansi::BOLD, ansi::CYAN, ansi::RESET,
    );

    // 调色板：每个字母使用一种前景色，并立即复位
    println!(
        "{}R{} {}G{} {}B{} {}M{} {}C{} {}Y{}",
        ansi::RED, ansi::RESET,
        ansi::GREEN, ansi::RESET,
        ansi::BLUE, ansi::RESET,
        ansi::MAGENTA, ansi::RESET,
        ansi::CYAN, ansi::RESET,
        ansi::YELLOW, ansi::RESET,
    );

    // 测试脚本据此判断程序正常结束
    println!("{}[WithColor]: ok!{}", ansi::GREEN, ansi::RESET);
}
```

同时把 `Cargo.toml` 中包名由模板的 `print_with_colord` 修正为 `print_with_color`，避免拼写错误进入构建产物名。

### 3.4 改动位置

| 文件 | 改动 |
| --- | --- |
| `exercises/print_with_color/Cargo.toml` | 包名改为 `print_with_color` |
| `exercises/print_with_color/src/main.rs` | 实现 ANSI 彩色输出 |
| `exercises/print_with_color/scripts/test-print.sh` | 本练习测试脚本 |
| `exercises/scripts/test-print.sh` | 统一入口包装脚本 |

### 3.5 编译与测试步骤

```bash
cd /path/to/arceos

# 方式一：直接编译并运行
make A=exercises/print_with_color LOG=warn QEMU_LOG=n PFLASH=n run

# 方式二：使用测试脚本（推荐）
cd exercises
./scripts/test-print.sh
# 或者
./print_with_color/scripts/test-print.sh
```

### 3.6 实验现象

串口输出（`\x1b` 代表 ESC 字节）：

```text
arch = riscv64
platform = riscv64-qemu-virt
target = riscv64gc-unknown-none-elf
smp = 1
build_mode = release
log_level = warn

\x1b[1m\x1b[32m[WithColor]\x1b[0m: \x1b[1m\x1b[36mHello, Arceos!\x1b[0m
\x1b[31mR\x1b[0m \x1b[32mG\x1b[0m \x1b[34mB\x1b[0m \x1b[35mM\x1b[0m \x1b[36mC\x1b[0m \x1b[33mY\x1b[0m
\x1b[32m[WithColor]: ok!\x1b[0m
```

在支持 ANSI 的终端中，可以看到 `[WithColor]` 为绿色加粗，`Hello, Arceos!` 为青色加粗，随后一行是六种颜色的字母。测试脚本检查的 PASS 项：

```text
PASS: found bold ANSI sequence
PASS: found green ANSI sequence
PASS: found cyan ANSI sequence
PASS: found ANSI reset sequence
PASS: found greeting text
PASS: found success marker
test-print.sh: ALL CHECKS PASSED
```

### 3.7 结果分析

- 每条 `println!` 产生完整的文本行，ANSI 序列嵌入在普通字节流中，控制台驱动无需理解颜色语义。
- 绿色标签与青色正文分别由独立的 SGR 序列设置：`ESC[1m`（加粗）和 `ESC[32m`（绿色）是两条序列，而不是 `ESC[1;32m`。测试脚本按实际字节序列校验，因此能够精确验证颜色确实被输出。
- 每段着色后立即输出 `ESC[0m`，避免后续内核日志继承颜色。
- 应用返回后，`axruntime` 在没有 `multitask` 特性时自动调用关机流程，QEMU 以退出码 0 结束。

### 3.8 踩坑总结

1. **测试断言必须匹配实际序列**：最初测试查找 `\x1b[1;32m`，但程序输出的是 `\x1b[1m\x1b[32m`，导致误报失败。修复方式是分别校验加粗和颜色序列。
2. **忘记复位会污染后续输出**：如果不输出 `ESC[0m`，后续所有内核日志都会带颜色。
3. **颜色由终端解释**：QEMU 只透传字节；如果将输出重定向到不支持 ANSI 的文件，需要用 `cat -v` 或十六进制查看器观察 ESC 字节。
4. **模板包名拼写错误**：`print_with_colord` 会被 `Makefile` 用作产物名，改为 `print_with_color` 后日志与测试更清晰。

---

## 4. 练习二：`ramfs_rename`

### 4.1 实验目的

在 ArceOS 的 RAMFS（Random Access Memory File System，内存文件系统）上实现“同一目录下的文件重命名”，掌握：

1. VFS（Virtual File System，虚拟文件系统）的节点抽象与 `rename` 语义；
2. RAMFS 目录的 `BTreeMap<String, Arc<dyn VfsNodeOps>>` 数据结构；
3. 挂载点路径的解析与文件系统路由；
4. 如何通过 crate 替换/补丁机制把本地实现注入 ArceOS 构建。

### 4.2 原理分析

ArceOS 的文件系统调用链为：

```text
std::fs::rename(old, new)
  -> arceos_api::fs::ax_rename
  -> axfs::root::rename
  -> RootDirectory::rename
  -> 目标文件系统 root_dir().rename(src_rest, dst_rest)
  -> RamFS DirNode::rename
```

RAMFS 的每个目录节点包含：

```rust
children: RwLock<BTreeMap<String, VfsNodeRef>>,
```

其中 `VfsNodeRef = Arc<dyn VfsNodeOps>`。重命名不需要复制文件内容，只需：

1. 解析源路径和目标路径，得到各自的父目录与文件名；
2. 要求两个父目录是同一个目录（本练习明确“仅支持重命名，不支持跨目录移动”）；
3. 从 `BTreeMap` 中 `remove(src_name)`，再 `insert(dst_name, node)`；
4. 目标已存在时返回 `AlreadyExists`（`axfs` 上层会在必要时先删除目标）。

挂载点路由由 `RootDirectory::resolve_fs` 完成：去掉前导 `/`，匹配最长的挂载路径前缀，得到 `(文件系统, 相对路径)`。如果源和目标解析到不同的文件系统，则拒绝重命名。

### 4.3 核心代码（逐行注释）

#### 4.3.1 RAMFS 目录重命名

文件：`axfs_ramfs/src/dir.rs`

```rust
/// Returns the `Arc` of this directory node.
fn this_arc(&self) -> VfsResult<Arc<DirNode>> {
    self.this.upgrade().ok_or(VfsError::NotFound) // Weak -> Arc
}

/// Resolves `path` to a directory node.
///
/// `self` is the directory on which `rename` was invoked.  For a mounted
/// filesystem this is its root, so both absolute and relative paths are
/// resolved from `self`.  Walking `parent()` must be avoided: for a mounted
/// root it points into the parent filesystem.
fn resolve_dir(&self, path: &str) -> VfsResult<Arc<DirNode>> {
    let base = self.this_arc()?;                       // 从当前目录出发
    let rel = path.strip_prefix('/').unwrap_or(path);  // 绝对路径视为 FS 根相对路径
    let rel = rel.trim_matches('/');
    if rel.is_empty() {
        return Ok(base);                               // "" 或 "/" 表示根目录
    }
    let node = base.lookup(rel)?;                      // 逐级查找
    let dir = node
        .as_any()
        .downcast_ref::<DirNode>()                     // 必须是目录节点
        .ok_or(VfsError::NotADirectory)?;
    dir.this_arc()
}

/// Splits `path` into its parent directory and its final component.
fn dir_and_name(&self, path: &str) -> VfsResult<(Arc<DirNode>, String)> {
    let path = path.trim_end_matches('/');
    let (dir_path, name) = match path.rfind('/') {     // 最后一个 '/' 之前是父目录
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => ("", path),
    };
    if name.is_empty() || name == "." || name == ".." {
        return Err(VfsError::InvalidInput);            // 非法文件名
    }
    let dir = if path.starts_with('/') || !dir_path.is_empty() {
        self.resolve_dir(dir_path)?                     // 解析父目录
    } else {
        self.this_arc()?                                // 当前目录
    };
    Ok((dir, String::from(name)))
}
```

真正的 `rename` 实现：

```rust
fn rename(&self, src_path: &str, dst_path: &str) -> VfsResult {
    log::debug!("rename at ramfs: {} -> {}", src_path, dst_path);

    // 1. 解析源/目标的父目录与文件名
    let (src_dir, src_name) = self.dir_and_name(src_path)?;
    let (dst_dir, dst_name) = self.dir_and_name(dst_path)?;

    // 2. 只允许同一目录内重命名；跨目录移动返回 Unsupported
    if !Arc::ptr_eq(&src_dir, &dst_dir) {
        return Err(VfsError::Unsupported);
    }
    if src_name == dst_name {                          // 重命名为自身是空操作
        return Ok(());
    }

    // 3. 取源节点（Arc 克隆，只增加引用计数，不复制文件数据）
    let node = src_dir
        .children
        .read()
        .get(&src_name)
        .cloned()
        .ok_or(VfsError::NotFound)?;

    // 4. 在目录项的 BTreeMap 中移动该 Arc
    let mut children = src_dir.children.write();
    if children.contains_key(&dst_name) {
        return Err(VfsError::AlreadyExists);           // 目标已存在
    }
    children.remove(&src_name);
    children.insert(dst_name, node);
    Ok(())
}
```

#### 4.3.2 挂载点路径解析与路由

文件：`modules/axfs/src/root.rs`

```rust
/// Resolves `path` to the filesystem that should serve it and returns the
/// path relative to that filesystem's root.
fn resolve_fs(&self, path: &str) -> (Arc<dyn VfsOps>, String) {
    let mut path = path.trim_matches('/');
    while let Some(rest) = path.strip_prefix("./") {   // 去掉开头的 "./"
        path = rest;
    }

    let mut idx = 0;
    let mut max_len = 0;
    // 选择最长前缀匹配的挂载点
    for (i, mp) in self.mounts.iter().enumerate() {
        if path.starts_with(&mp.path[1..]) && mp.path.len() - 1 > max_len {
            max_len = mp.path.len() - 1;
            idx = i;
        }
    }

    if max_len == 0 {
        (self.main_fs.clone(), String::from(path))     // 落在主文件系统
    } else {
        (self.mounts[idx].fs.clone(), String::from(&path[max_len..])) // 剩余路径
    }
}

fn rename(&self, src_path: &str, dst_path: &str) -> VfsResult {
    let (src_fs, src_rest) = self.resolve_fs(src_path); // 源文件系统 + 相对路径
    let (dst_fs, dst_rest) = self.resolve_fs(dst_path); // 目标文件系统 + 相对路径

    if !Arc::ptr_eq(&src_fs, &dst_fs) {
        return ax_err!(PermissionDenied, "cannot rename across filesystems");
    }
    if src_rest.is_empty() || dst_rest.is_empty() {
        return ax_err!(PermissionDenied);               // 不允许重命名挂载点
    }
    src_fs.root_dir().rename(&src_rest, &dst_rest)       // 交给具体文件系统
}
```

#### 4.3.3 使用本地 RAMFS 实现

文件：`Cargo.toml`

```toml
[patch.crates-io]
kernel_guard = { path = "../crates/kernel_guard" }
axfs_ramfs = { path = "axfs_ramfs" }   # 用仓库内的实现替换 crates.io 版本
```

`axfs_ramfs/Cargo.toml` 中的版本号同步改为锁文件中的 `0.1.2`，使 Cargo 的 patch 生效。

### 4.4 改动位置

| 文件 | 改动 |
| --- | --- |
| `axfs_ramfs/src/dir.rs` | 新增 `this_arc`、`resolve_dir`、`dir_and_name` 与 `rename` |
| `modules/axfs/src/root.rs` | 抽出 `resolve_fs`，修复 `rename` 的源/目标挂载路径处理 |
| `Cargo.toml` | 增加 `[patch.crates-io] axfs_ramfs` |
| `axfs_ramfs/Cargo.toml` | 版本改为 `0.1.2` 以匹配锁文件 |
| `exercises/ramfs_rename/scripts/test-ramfs_rename.sh` | 测试脚本 |
| `exercises/scripts/test-ramfs_rename.sh` | 统一入口包装脚本 |

### 4.5 编译与测试步骤

```bash
cd /path/to/arceos

# 需要一个块设备：myfs 模式下 Disk 参数仍会被 axfs 使用，故使用内置 RAM 磁盘
make A=exercises/ramfs_rename FEATURES=driver-ramdisk \
     LOG=warn QEMU_LOG=n PFLASH=n run

# 测试脚本
cd exercises
./scripts/test-ramfs_rename.sh
```

### 4.6 实验现象

```text
Create '/tmp/f1' and write [hello] ...
Rename '/tmp/f1' to '/tmp/f2' ...
Read '/tmp/f2' content: [hello] ok!

[Ramfs-Rename]: ok!
```

测试脚本结果：

```text
PASS: found file creation
PASS: found rename request
PASS: found content after rename
PASS: found success marker
test-ramfs_rename.sh: ALL CHECKS PASSED
```

### 4.7 结果分析

- `/tmp` 由 `axfs` 挂载一个独立的 RAMFS 实例；`/tmp/f1` 被创建后，文件内容为 `hello`。
- `fs::rename("/tmp/f1", "/tmp/f2")` 经过 VFS 路由，`resolve_fs` 将两条路径都解析到 `/tmp` 所属的 RAMFS，相对路径分别为 `/f1` 与 `/f2`。
- RAMFS 根目录在同一 `BTreeMap` 中把键 `f1` 改为 `f2`，节点 `Arc` 与文件内容完全保持不变，因此再次读取 `/tmp/f2` 仍得到 `hello`。
- 重命名操作的时间复杂度为 `O(log n)`，仅修改目录项，不搬移数据。

### 4.8 踩坑总结

1. **crate patch 版本必须匹配**：最初本地 `axfs_ramfs` 版本为 `0.1.1`，而 `Cargo.lock` 锁定 `0.1.2`，Cargo 提示 “Patch was not used in the crate graph”。把本地版本改为 `0.1.2` 后生效。
2. **不能通过 `parent()` 找文件系统根**：挂载根节点的 `parent()` 指向父文件系统中的挂载点，向上走会逃出当前 RAMFS。因此 `resolve_dir` 直接把当前节点当作文件系统根来解析路径。
3. **`axfs` 的 `rename` 只剥离了源路径的挂载前缀**：模板代码把目标路径 `/tmp/f2` 原样传给 RAMFS，RAMFS 会误以为要移动到 `tmp` 子目录。修复为对源和目标都调用 `resolve_fs`，并检查二者属于同一文件系统。
4. **`myfs` 仍要求块设备**：即使主文件系统是 RAMFS，`axfs::init_filesystems` 仍会 `take_one()` 块设备。使用 `driver-ramdisk` 特性提供一个内置 16 MiB 内存磁盘即可。
5. **目标已存在的处理**：`axfs::root::rename` 会先尝试删除目标文件；RAMFS 的 `rename` 也检查 `AlreadyExists`，避免静默覆盖。

---

## 5. 练习三：`alt_alloc`

### 5.1 实验目的

替换 ArceOS 的全局内存分配器，实现并启用基于双端 bump（指针递增/递减）的早期分配器，验证 `Vec` 的大批量分配、排序与释放，掌握：

1. Rust `GlobalAlloc` 与 ArceOS `GlobalAllocator` 的关系；
2. `BaseAllocator`、`ByteAllocator`、`PageAllocator` 三个 trait 的职责；
3. 双端 bump 分配器的空间布局与“全有或全无”释放策略；
4. 页分配的对齐语义与 `dealloc_pages` 的限制。

### 5.2 原理分析

`EarlyAllocator` 把一段连续内存分成三部分：

```text
[ 已分配字节区 | 可用空洞 | 已分配页区 ]
start        b_pos      p_pos        end
             --->        <---
```

- 字节分配：从 `b_pos` 向高地址增长；
- 页分配：从 `p_pos` 向低地址增长，每次按页大小和对齐向下取整；
- 两者相遇时分配失败。

释放策略：

- 每个字节分配成功时 `b_count += 1`；
- `dealloc` 只递减计数；当计数归零时，一次性把 `b_pos` 重置到 `start`，回收整个字节区；
- 页分配不回收：`p_pos` 只降不升，且字节区可能已经越过页区边界，无法安全合并。

`alt_axalloc::GlobalAllocator` 用 `SpinNoIrq<EarlyAllocator<PAGE_SIZE>>` 保护内部状态，实现 `GlobalAlloc`，并通过 `#[global_allocator]` 注册为全局分配器。`axruntime` 在 `feature = "alt_alloc"` 时调用 `alt_axalloc::global_init`，而不是 `axalloc::global_init`。

### 5.3 核心代码（逐行注释）

文件：`modules/bump_allocator/src/lib.rs`

```rust
pub struct EarlyAllocator<const SIZE: usize> {
    start: usize,   // 管理区间起始地址
    end: usize,     // 管理区间结束地址（不含）
    b_pos: usize,   // 字节区当前 bump 指针
    b_count: usize, // 尚未释放的字节分配数量
    p_pos: usize,   // 页区当前底部地址
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        assert!(SIZE.is_power_of_two());
        let start = align_up(start, SIZE);                       // 页对齐起始
        let end = align_down(start.saturating_add(size), SIZE);  // 页对齐结束
        assert!(end > start);
        self.start = start;
        self.end = end;
        self.b_pos = start;      // 字节区从低端开始
        self.b_count = 0;
        self.p_pos = end;        // 页区从高端开始
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        let start = align_up(start, SIZE);
        let end = align_down(start.saturating_add(size), SIZE);
        if start != self.end || end <= start {
            return Err(AllocError::NoMemory);   // 只接受紧邻的高地址扩展
        }
        self.end = end;
        self.p_pos = end;                       // 新页区仍在最高端
        Ok(())
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let align = layout.align();
        if !align.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }
        let pos = align_up(self.b_pos, align);              // 满足对齐要求
        let new_pos = pos.checked_add(layout.size())
            .ok_or(AllocError::NoMemory)?;
        if new_pos > self.p_pos {                            // 不能撞上页区
            return Err(AllocError::NoMemory);
        }
        self.b_pos = new_pos;                                // 前进
        self.b_count += 1;                                   // 记录一次分配
        NonNull::new(pos as *mut u8).ok_or(AllocError::NoMemory)
    }

    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        if self.b_count > 0 {
            self.b_count -= 1;
            if self.b_count == 0 {
                self.b_pos = self.start;                     // 全有或全无地回收
            }
        }
    }

    fn total_bytes(&self) -> usize { self.end - self.start }
    fn used_bytes(&self) -> usize { self.b_pos - self.start }
    fn available_bytes(&self) -> usize { self.p_pos - self.b_pos }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if num_pages == 0 || !align_pow2.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }
        let align = align_pow2.max(SIZE);                    // 至少按页对齐
        let size = num_pages.checked_mul(SIZE)
            .ok_or(AllocError::NoMemory)?;
        let top = self.p_pos.checked_sub(size)
            .ok_or(AllocError::NoMemory)?;
        let pos = align_down(top, align);                    // 向下对齐
        if pos < self.b_pos {                                // 与字节区相交
            return Err(AllocError::NoMemory);
        }
        self.p_pos = pos;
        Ok(pos)
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // 早期分配器不回收页：b_pos 可能已越过可回退的位置
    }

    fn total_pages(&self) -> usize { (self.end - self.start) / SIZE }
    fn used_pages(&self) -> usize { (self.end - self.p_pos) / SIZE }
    fn available_pages(&self) -> usize { (self.p_pos - self.b_pos) / SIZE }
}
```

文件：`modules/alt_axalloc/src/lib.rs`

```rust
pub fn add_memory(&self, start_vaddr: usize, size: usize) -> AllocResult {
    self.inner.lock().add_memory(start_vaddr, size)  // 之前是 unimplemented!()
}

pub fn global_add_memory(start_vaddr: usize, size: usize) -> AllocResult {
    GLOBAL_ALLOCATOR.add_memory(start_vaddr, size)   // 之前是 unimplemented!()
}
```

`name()` 返回 `"early (bump)"`，便于在启动日志中确认替换成功。

### 5.4 改动位置

| 文件 | 改动 |
| --- | --- |
| `modules/bump_allocator/src/lib.rs` | 实现双端 bump 分配器 |
| `modules/alt_axalloc/src/lib.rs` | 实现 `add_memory`、`global_add_memory`，更新名称 |
| `exercises/alt_alloc/scripts/test-alt_alloc.sh` | 测试脚本 |
| `exercises/scripts/test-alt_alloc.sh` | 统一入口包装脚本 |

应用本身 `exercises/alt_alloc/src/main.rs` 保持模板逻辑：分配 300 万个 `u32` 的 `Vec`、排序并检查有序性。

### 5.5 编译与测试步骤

```bash
cd /path/to/arceos

# LOG=info 可看到 allocator 名称
make A=exercises/alt_alloc LOG=info QEMU_LOG=n PFLASH=n run

# 测试脚本
cd exercises
./scripts/test-alt_alloc.sh
```

### 5.6 实验现象

```text
Initialize global memory allocator...
  use early (bump) allocator.
...
Running bump tests...
Bump tests run OK!
Shutting down...
```

测试脚本结果：

```text
PASS: found alternate allocator selection
PASS: found test start
PASS: found success marker
test-alt_alloc.sh: ALL CHECKS PASSED
```

### 5.7 结果分析

- 启动日志明确显示使用 `early (bump)` 分配器，说明 `axruntime` 的 `alt_alloc` 分支生效。
- `Vec::with_capacity(3_000_000)` 请求约 12 MiB 连续内存；bump 分配器直接向前移动 `b_pos`，无需空闲链表搜索，分配速度常数级。
- `v.push` 不会触发扩容（容量已足够）；`v.sort()` 使用 Rust 标准库的稳定排序，可能额外分配约一半长度的临时缓冲区，总占用仍远小于 128 MiB。
- 排序后逐对比较 `v[i] <= v[i + 1]`，验证了内存内容正确。
- 程序结束时 `Vec` 析构，`b_count` 归零，字节区整体回收；页区在本应用中未参与实际回收。

### 5.8 踩坑总结

1. **`align_pow2` 是字节对齐值而不是指数**：`BitmapPageAllocator` 的实现先检查 `align_pow2 % PAGE_SIZE == 0` 再除以页大小求 `log2`，因此分配器必须把 `PAGE_SIZE` 当作对齐值处理。
2. **页区不能回收**：如果 `dealloc_pages` 试图把 `p_pos` 上移，可能覆盖仍然存活的字节分配；因此实现为空操作，并在注释中说明。
3. **字节区“全有或全无”**：只维护分配计数，不维护每个块的边界；只有全部释放时才回退 `b_pos`，这是早期 bump 分配器的典型取舍。
4. **`add_memory` 不能 `unimplemented!()`**：`axruntime` 会对除最大空闲区之外的每个空闲区调用 `global_add_memory`；虽然 QEMU `virt` 当前只有一个空闲区，但实现为“仅接受紧邻扩展”更稳健。
5. **特性组合**：`axstd` 的 `alt_alloc` 特性并不会自动开启 `axstd/alloc`；应用通过 `extern crate alloc` 直接使用 `alloc` crate，全局分配器由 `alt_axalloc` 提供。

---

## 6. 练习四：`support_hashmap`

### 6.1 实验目的

让 `no_std` 的 ArceOS 内核应用可以使用 `HashMap`，完成 50,000 个字符串键的插入与遍历校验，掌握：

1. `no_std` 环境缺少 `std::collections::HashMap` 的原因；
2. `hashbrown` 作为 `no_std` 哈希表的接入方式；
3. 默认哈希器（hasher）在无标准库、无随机源环境下的实现；
4. `axstd` 的模块再导出与特性门控。

### 6.2 原理分析

Rust 的 `alloc` crate 只提供 `BTreeMap`、`BTreeSet`、`VecDeque` 等基于比较的容器，不提供哈希表；`HashMap` 位于 `std` 中。`hashbrown` 是 Rust 标准库哈希表的底层实现，可独立用于 `no_std + alloc` 环境。

`hashbrown` 0.15 的 `HashMap<K, V, S = DefaultHashBuilder>` 要求：

- 开启 `default-hasher` 特性时，`DefaultHashBuilder = foldhash::fast::RandomState`；
- `foldhash` 的 `RandomState` 在关闭 `std` 后仍可用，它使用编译期常量和原子计数器生成种子，不依赖操作系统随机数；
- `HashMap::new()` 只在 `default-hasher` 特性下提供。

接入方式：在 `axstd` 中新增 `hashbrown` 依赖，并把原来对 `alloc::collections` 的再导出改成一个自定义 `collections` 模块，同时导出 `alloc::collections::*`、`HashMap` 和 `HashSet`。这样应用写 `use std::collections::HashMap;`（`std` 是 `axstd` 的别名）即可获得哈希表。

### 6.3 核心代码（逐行注释）

文件：`ulib/axstd/Cargo.toml`

```toml
# no_std 哈希表；default-hasher 使用 foldhash 的 RandomState
hashbrown = { version = "0.15", default-features = false, features = ["default-hasher"] }
```

文件：`ulib/axstd/src/lib.rs`

```rust
#[cfg(feature = "alloc")]
#[doc(no_inline)]
pub use alloc::{boxed, format, string, vec};   // 不再直接再导出 alloc::collections

/// Collection types.
///
/// 除 alloc 的 VecDeque/BTreeMap 外，还提供基于 hashbrown 的 HashMap/HashSet。
#[cfg(feature = "alloc")]
pub mod collections {
    pub use alloc::collections::*;            // BTreeMap、VecDeque 等
    pub use hashbrown::{HashMap, HashSet};    // 新增的哈希容器

    pub mod hash_map {
        pub use hashbrown::hash_map::*;       // Entry、Iter 等
    }
    pub mod hash_set {
        pub use hashbrown::hash_set::*;
    }
}
```

文件：`exercises/support_hashmap/src/main.rs`（模板逻辑，无需修改）

```rust
use std::collections::HashMap;                // std 是 axstd 的别名

fn test_hashmap() {
    const N: u32 = 50_000;
    let mut m = HashMap::new();
    for value in 0..N {
        let key = format!("key_{value}");     // 50,000 个 String 键
        m.insert(key, value);                 // 平均 O(1) 插入
    }
    for (k, v) in m.iter() {
        if let Some(k) = k.strip_prefix("key_") {
            assert_eq!(k.parse::<u32>().unwrap(), *v);  // 校验键值对应
        }
    }
    println!("test_hashmap() OK!");
}
```

### 6.4 改动位置

| 文件 | 改动 |
| --- | --- |
| `ulib/axstd/Cargo.toml` | 新增 `hashbrown` 依赖及 `default-hasher` 特性 |
| `ulib/axstd/src/lib.rs` | 新增 `collections` 模块，导出 `HashMap`/`HashSet` |
| `Cargo.lock` | 新增 `foldhash` 依赖记录 |
| `exercises/support_hashmap/scripts/test-support_hashmap.sh` | 测试脚本 |
| `exercises/scripts/test-support_hashmap.sh` | 统一入口包装脚本 |

### 6.5 编译与测试步骤

```bash
cd /path/to/arceos

make A=exercises/support_hashmap LOG=warn QEMU_LOG=n PFLASH=n run

cd exercises
./scripts/test-support_hashmap.sh
```

### 6.6 实验现象

```text
Running memory tests...
test_hashmap() OK!
Memory tests run OK!
```

测试脚本结果：

```text
PASS: found test start
PASS: found HashMap verification
PASS: found success marker
test-support_hashmap.sh: ALL CHECKS PASSED
```

### 6.7 结果分析

- `hashbrown::HashMap` 在 `no_std + alloc` 环境下正常工作，说明 `foldhash` 的默认哈希器不依赖 `std` 或操作系统随机源。
- 50,000 次插入和 50,000 次遍历全部通过断言；`format!` 产生的 `String` 键由 `axalloc` 的全局分配器分配。
- 哈希表平均插入/查找复杂度为 `O(1)`，相比 `BTreeMap` 的 `O(log n)` 更适合大规模键值集合。
- `axstd::collections` 同时保留原有 `alloc` 容器，现有代码不受影响。

### 6.8 踩坑总结

1. **不能简单再导出 `hashbrown::HashMap` 的默认类型而关闭 `default-hasher`**：关闭该特性后 `DefaultHashBuilder` 是一个空枚举，`HashMap::new()` 不存在；必须开启 `default-hasher`，或为自定义 hasher 实现 `Default` 并改用 `with_hasher`。
2. **`foldhash` 是新增传递依赖**：本机 Cargo 的清华镜像源把同一个 crate 缓存在两个不同的 registry 哈希目录下，`--offline` 构建时出现 “failed to download foldhash”；把 `.crate` 文件复制到当前 registry 缓存后恢复。正常联网环境会自动下载，`Cargo.lock` 已固定 `foldhash 0.1.5`。
3. **替换再导出而不是叠加**：必须先移除 `pub use alloc::{..., collections, ...}` 中的 `collections`，否则与自定义 `pub mod collections` 重名冲突。
4. **特性门控**：`collections` 模块依赖全局分配器，因此以 `feature = "alloc"` 门控；只启用 `alt_alloc` 的应用不会编译该模块。

---

## 7. 练习五：`sys_map`

### 7.1 实验目的

在 ArceOS 的单体内核应用中实现 `mmap` 系统调用，使用户程序能够把文件映射到自己的地址空间并直接读取文件内容，掌握：

1. 用户态 `ecall` 陷入、`TrapFrame` 与系统调用分发；
2. ELF（Executable and Linkable Format，可执行与可链接格式）程序加载与用户地址空间构建；
3. `mmap` 的地址选择、页映射与权限保护；
4. FAT32 文件系统镜像的制作与 `virtio-blk` 块设备挂载；
5. 文件描述符、`read`/`lseek`/`close` 等系统调用的协作。

### 7.2 原理分析

#### 7.2.1 执行流程

```text
内核 main
  -> axmm::new_user_aspace()                创建用户地址空间（复制内核高地址映射）
  -> load_user_app("/sbin/mapfile")         解析 ELF，映射 PT_LOAD（Program Table Load，可加载段）
  -> init_user_stack()                      在地址空间顶端映射用户栈并写入 argv
  -> spawn_user_task()                      创建带独立页表的任务
  -> UspaceContext::enter_uspace()          sret 进入 U-mode
用户程序
  -> openat("/tmp/test_file", O_CREAT...)   创建文件
  -> write()                                写入 "hello, arceos!\0"
  -> openat("/tmp/test_file", O_RDONLY)     重新只读打开
  -> mmap(NULL, 32, PROT_READ, MAP_PRIVATE, fd, 0)
  -> 读取映射区并输出
  -> exit(0)
内核
  -> join 用户任务，打印退出码，关机
```

#### 7.2.2 `mmap` 语义

`mmap` 的参数：

```text
void *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset);
```

本实现支持：

- `addr == NULL`：由内核选择空闲地址；
- `MAP_FIXED`：使用调用者给出的地址（页对齐）；
- `MAP_ANONYMOUS` 或 `fd < 0`：匿名零页映射；
- 文件映射：`MAP_PRIVATE`，将文件内容复制到新分配的物理页，用户写入不会回写文件；
- `PROT_READ`/`PROT_WRITE`/`PROT_EXEC` 转换为 ArceOS `MappingFlags`。

下文中 VA（Virtual Address，虚拟地址）表示进程地址空间中的地址，PA（Physical Address，物理地址）表示内存总线上的地址，PTE（Page Table Entry，页表项）表示页表中的一项。

文件映射的关键步骤：

1. 将 `length` 向上取整到 4 KiB 页大小；
2. 在当前任务地址空间中寻找空闲区域；
3. 以 `READ | WRITE | USER` 权限、`populate=true` 建立映射，确保物理页立即分配并清零；
4. `lseek` 到文件偏移，`read` 文件内容到内核缓冲区；
5. 通过 `AddrSpace::write` 把内核缓冲区复制到用户虚拟地址对应的物理页；
6. 若用户未请求写权限，用 `AddrSpace::protect` 把页改为只读；
7. 返回映射起始地址。

#### 7.2.3 系统调用分发

ArceOS 使用 `linkme` 的分布式切片注册系统调用处理函数：

```rust
#[register_trap_handler(SYSCALL)]
fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> isize
```

RISC-V 用户态执行 `ecall` 后，硬件进入 S-mode，`axhal` 的 trap 分发器读取 `a7` 作为系统调用号，调用该处理函数；处理函数从 `tf.arg0()`…`tf.arg5()` 取参数，返回值写回 `a0`。

### 7.3 核心代码（逐行注释）

#### 7.3.1 `mmap` 系统调用

文件：`exercises/sys_map/src/syscall.rs`

```rust
/// NULL 地址时的默认映射提示地址，避开低地址的 ELF 和顶部的用户栈
const MMAP_BASE: usize = 0x2000_0000;

/// ArceOS 错误 -> 用户程序看到的负 errno
fn errno(e: axerrno::AxError) -> isize {
    -(LinuxError::from(e).code() as isize)
}

fn sys_mmap(
    addr: *mut usize,   // 请求地址（可为 NULL）
    length: usize,      // 映射长度（字节）
    prot: i32,          // PROT_READ / PROT_WRITE / PROT_EXEC
    flags: i32,         // MAP_PRIVATE / MAP_ANONYMOUS / MAP_FIXED ...
    fd: i32,            // 文件描述符，匿名映射时为 -1
    offset: isize,      // 文件偏移
) -> isize {
    // 1. 解析权限与标志
    let Some(prot) = MmapProt::from_bits(prot) else {
        return -LinuxError::EINVAL.code() as isize;
    };
    let Some(flags) = MmapFlags::from_bits(flags) else {
        return -LinuxError::EINVAL.code() as isize;
    };
    if length == 0 {
        return -LinuxError::EINVAL.code() as isize;
    }

    // 2. 长度按页向上取整
    let Some(size) = length
        .checked_add(PAGE_SIZE_4K - 1)
        .map(|len| len & !(PAGE_SIZE_4K - 1))
    else {
        return -LinuxError::ENOMEM.code() as isize;
    };

    // 3. 取得当前任务的用户地址空间
    let curr = current();
    let aspace = curr.task_ext().aspace.clone();
    let mut aspace = aspace.lock();

    // 4. 决定映射地址：MAP_FIXED 用请求地址，否则从 hint 开始找空闲区
    let req_addr = addr as usize;
    let hint = if req_addr != 0 {
        VirtAddr::from_usize(req_addr).align_down_4k()
    } else {
        VirtAddr::from_usize(MMAP_BASE)
    };
    let vaddr = if flags.contains(MmapFlags::MAP_FIXED) {
        if req_addr == 0 || !aspace.contains_range(hint, size) {
            return -LinuxError::EINVAL.code() as isize;
        }
        hint
    } else {
        let limit = VirtAddrRange::from_start_size(aspace.base(), aspace.size());
        match aspace.find_free_area(hint, size, limit) {
            Some(vaddr) => vaddr,
            None => return -LinuxError::ENOMEM.code() as isize,
        }
    };

    // 5. 匿名映射：直接建立零页映射
    let anonymous = flags.contains(MmapFlags::MAP_ANONYMOUS) || fd < 0;
    if anonymous {
        let mut map_flags = MappingFlags::from(prot);
        if !map_flags.contains(MappingFlags::READ) {
            // RISC-V 叶 PTE 必须 R=1；无权限 PTE 会被当成下一级页表指针
            map_flags |= MappingFlags::READ;
        }
        if let Err(e) = aspace.map_alloc(vaddr, size, map_flags, true) {
            return errno(e);
        }
        return vaddr.as_usize() as isize;
    }

    // 6. 文件映射：先映射为可写，便于内核填充文件内容
    if let Err(e) = aspace.map_alloc(
        vaddr,
        size,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        true,                                   // populate：立即分配物理页
    ) {
        return errno(e);
    }

    // 7. 文件定位与读取
    let seek_ret = api::sys_lseek(fd, offset as _, 0);
    if seek_ret < 0 {
        let _ = aspace.unmap(vaddr, size);
        return seek_ret as isize;
    }
    let mut buf = vec![0u8; size];              // 内核缓冲区
    let n = api::sys_read(fd, buf.as_mut_ptr() as *mut c_void, size);
    if n < 0 {
        let _ = aspace.unmap(vaddr, size);
        return n;
    }

    // 8. 把文件内容复制到用户页
    if let Err(e) = aspace.write(vaddr, &buf[..n as usize]) {
        let _ = aspace.unmap(vaddr, size);
        return errno(e);
    }

    // 9. 应用最终权限（例如只读），并刷新 TLB
    let mut final_flags = MappingFlags::from(prot);
    if !final_flags.contains(MappingFlags::READ) {
        final_flags |= MappingFlags::READ;
    }
    if let Err(e) = aspace.protect(vaddr, size, final_flags) {
        let _ = aspace.unmap(vaddr, size);
        return errno(e);
    }

    ax_println!(
        "sys_mmap: file map at {:#x}, copied {} byte(s)",
        vaddr, n
    );
    vaddr.as_usize() as isize
}
```

#### 7.3.2 系统调用号与分发

```rust
const SYS_OPENAT: usize = 56;    // openat(dirfd, path, flags, mode)
const SYS_CLOSE: usize = 57;     // close(fd)
const SYS_READ: usize = 63;      // read(fd, buf, count)
const SYS_WRITE: usize = 64;     // write(fd, buf, count)
const SYS_WRITEV: usize = 66;    // writev(fd, iov, iocnt)
const SYS_EXIT: usize = 93;      // exit(code)
const SYS_EXIT_GROUP: usize = 94; // exit_group(code)
const SYS_MMAP: usize = 222;     // mmap(addr, len, prot, flags, fd, offset)

#[register_trap_handler(SYSCALL)]
fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> isize {
    let ret = match syscall_num {
        SYS_OPENAT => sys_openat(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _, tf.arg3() as _),
        SYS_CLOSE  => sys_close(tf.arg0() as _),
        SYS_READ   => sys_read(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_WRITE  => sys_write(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_WRITEV => sys_writev(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_MMAP   => sys_mmap(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _,
                               tf.arg3() as _, tf.arg4() as _, tf.arg5() as _),
        SYS_EXIT | SYS_EXIT_GROUP => axtask::exit(tf.arg0() as _),
        _ => -LinuxError::ENOSYS.code() as _,
    };
    ret
}
```

#### 7.3.3 用户态测试程序

文件：`payload/mapfile/src/main.rs`（无 libc，直接 `ecall`）

```rust
const SYS_OPENAT: usize = 56;
const SYS_CLOSE:  usize = 57;
const SYS_WRITE:  usize = 64;
const SYS_EXIT:   usize = 93;
const SYS_MMAP:   usize = 222;

const FILE_PATH: &[u8] = b"/tmp/test_file\0";
const CONTENT:   &[u8] = b"hello, arceos!";

// 通用系统调用封装：编号放 a7，参数放 a0..a5，返回值在 a0
unsafe fn syscall6(num: usize, a0: usize, a1: usize, a2: usize,
                   a3: usize, a4: usize, a5: usize) -> isize {
    let ret: isize;
    asm!("ecall",
         inlateout("a0") a0 => ret,
         in("a1") a1, in("a2") a2, in("a3") a3,
         in("a4") a4, in("a5") a5, in("a7") num,
         options(nostack));
    ret
}

// _start：创建文件、写入内容、重新打开、mmap、读取映射区、退出
let fd = syscall4(SYS_OPENAT, AT_FDCWD, FILE_PATH.as_ptr() as usize,
                  O_WRONLY | O_CREAT | O_TRUNC, 0o600);
write_all(fd, CONTENT);              // "hello, arceos!"
write_all(fd, b"\0");                // 以 NUL 结尾，方便 C 风格打印
syscall1(SYS_CLOSE, fd);

let fd = syscall4(SYS_OPENAT, AT_FDCWD, FILE_PATH.as_ptr() as usize, O_RDONLY, 0);
let addr = syscall6(SYS_MMAP, 0, 32, PROT_READ, MAP_PRIVATE, fd, 0);
// 映射区内容应为 "hello, arceos!"
```

实际实现先把 `Read back content: `、映射内容和换行符复制到同一个栈缓冲区，再调用一次 `write`。这样内核在每次系统调用前打印的 `handle_syscall [N] ...` 日志不会把整行拆开，官方测试脚本可以直接按整行匹配 `Read back content: hello, arceos!`。

#### 7.3.4 FAT32 镜像生成

由于环境中没有 `mtools`（`mcopy`/`mmd`），新增 `exercises/tools/mkdisk.py` 直接构造 FAT32：

- 512 字节扇区、每簇 1 个扇区、32 个保留扇区、2 个 FAT（File Allocation Table，文件分配表）；
- 引导扇区填写 BPB（BIOS Parameter Block，BIOS 参数块）、FAT32 扩展字段、卷标与 `0x55AA` 签名；
- FSInfo 扇区填写空闲簇数与下一个空闲簇；
- 根目录簇为 2，`/sbin` 为子目录，文件按簇链写入；
- 使用 8.3 短文件名（`MAPFILE`、`SKERNEL2`），无需 LFN（Long File Name，长文件名）；
- 64 MiB 镜像的簇数超过 65525，满足 `rust-fatfs` 对 FAT32 的判定条件。

### 7.4 改动位置

| 文件 | 改动 |
| --- | --- |
| `exercises/sys_map/src/syscall.rs` | 实现 `sys_mmap`，注册 `SYS_MMAP` 分发，补全错误码转换 |
| `exercises/sys_map/src/main.rs` | 用户任务结束后打印标记并关机 |
| `payload/mapfile/src/main.rs` | 新增无 libc 的用户态测试程序 |
| `payload/mapfile/linker.lds` | 用户程序链接脚本（代码段 0x10000，三个 PT_LOAD） |
| `payload/mapfile/Makefile` | 用 `rustc` 直接构建 freestanding ELF |
| `exercises/tools/mkdisk.py` | 新增 FAT32 镜像生成器 |
| `exercises/sys_map/scripts/test-sys_map.sh` | 测试脚本 |
| `exercises/scripts/test-sys_map.sh` | 统一入口包装脚本 |

### 7.5 编译与测试步骤

```bash
cd /path/to/arceos

# 1. 构建用户态 payload（ELF）
make -C payload/mapfile

# 2. 生成包含 /sbin/mapfile 的 FAT32 镜像
python3 exercises/tools/mkdisk.py exercises/sys_map/disk.img \
        /sbin/mapfile="$PWD/payload/mapfile/mapfile.elf"

# 3. 启动内核（需要 virtio-blk）
make A=exercises/sys_map BLK=y DISK_IMG="$PWD/exercises/sys_map/disk.img" \
     LOG=warn QEMU_LOG=n PFLASH=n run

# 4. 测试脚本
cd exercises
./scripts/test-sys_map.sh
```

### 7.6 实验现象

```text
phdr: offset: 0x1000=>0x10000 size: 0x1AA=>0x1AA
phdr: offset: 0x2000=>0x11000 size: 0x90=>0x90
entry: 0x1004e
Mapping user stack: VA:0x3fffff0000 -> VA:0x4000000000
MapFile ...
handle_syscall [222] ...
sys_mmap: addr=0x0, len=0x20, prot=0x1, flags=0x2, fd=3, off=0x0
sys_mmap: file map at VA:0x20000000, copied 15 byte(s)
Read back content: hello, arceos!
MapFile ok!
[SYS_EXIT]: system is exiting ..
monolithic kernel exit [Some(0)] normally!
[SysMap]: ok!
```

内核在每个系统调用前打印 `handle_syscall [N] ...`；用户程序把整行输出放在一次 `write` 中，因此内核日志只会出现在整行之前或之后，不会插入到 `Read back content:` 与 `hello, arceos!` 之间。

测试脚本结果：

```text
PASS: found user program start
PASS: found mmap syscall dispatched
PASS: found file-backed mapping
PASS: found mapped content printed
PASS: found file contents read through mmap
PASS: found user program success marker
PASS: found user process exit code
PASS: found kernel success marker
test-sys_map.sh: ALL CHECKS PASSED
```

### 7.7 结果分析

- ELF 加载器正确解析两个 `PT_LOAD` 段，将代码映射到 `0x10000`、只读数据映射到 `0x11000`，入口点为 `0x1004e`。
- 用户栈位于地址空间顶端 `0x3fffff0000`–`0x4000000000`，与 `mmap` 默认地址 `0x20000000` 不冲突。
- `sys_mmap` 在 `0x20000000` 找到空闲区，为 32 字节请求建立一页映射；文件实际内容 15 字节（14 个字符加 NUL），复制后其余页保持零。
- 最终权限被改为 `PROT_READ | USER`，用户程序只能读取；`AddrSpace::protect` 触发 TLB 刷新，保证新权限立即生效。
- 用户程序成功打印 `hello, arceos!`，说明文件内容确实通过 `mmap` 出现在用户地址空间中，而不是普通 `read`。
- 用户任务以 `0` 退出，内核 `join` 得到 `Some(0)`，随后关机。

### 7.8 踩坑总结

1. **缺少 `riscv64-linux-musl-gcc`**：模板中的 `mapfile.c` 依赖 musl 静态 libc，但本机没有该交叉编译器。新增 Rust `no_std` 直写 `ecall` 的 payload，并让 `payload/mapfile_c/Makefile` 在检测不到 musl 编译器时回退到它；`hello_c`/`fileops_c` 在无编译器时打印警告并跳过，使官方 `make payload` 仍返回成功。完整覆盖 `openat/write/close/mmap/exit` 路径。
2. **缺少 `mtools`**：`make disk_img` 使用 `mcopy`，本机未安装且无免密 `sudo`。新增纯 Python 的 `mkdisk.py`，不依赖外部工具。
3. **FAT32 判定条件**：最初使用 4 KiB 簇，64 MiB 镜像只有约 1.6 万簇，`rust-fatfs` 按簇数判定为 FAT16，报 “result of FAT32 determination ... differs”。改用 512 字节簇后簇数约 13 万，满足 FAT32 条件。
4. **`rust-fatfs` 的 “Is a directory” 日志**：挂载阶段打印若干条该日志，但它不是错误，文件系统仍正常工作；测试脚本不能把它当作失败。
5. **必须先映射可写再保护**：如果直接用 `PROT_READ` 填充文件内容，内核写入会触发页错误；实现为先映射 `READ|WRITE|USER`、复制内容，再用 `protect` 降为只读。
6. **RISC-V 叶 PTE 权限限制**：`R=0, W=1` 是保留编码；完全没有 `R/W/X` 的 PTE 会被解释为下一级页表指针。因此 `mmap` 至少保留 `READ` 位。
7. **日志交织**：内核在每次系统调用前打印日志。若用户程序分三次 `write` 输出前缀、内容和换行，整行会被内核日志拆开；改为先在栈缓冲区拼好整行、再调用一次 `write`，官方测试即可按完整行匹配。
8. **地址选择**：`mmap` 默认地址必须避开低地址的 ELF 和顶部的用户栈；选择 `0x2000_0000`，并通过 `find_free_area` 做最终确认。
9. **当前实现的范围**：文件映射按私有副本处理（`MAP_PRIVATE` 语义），未实现 `MAP_SHARED` 回写；未强制检查 `offset` 的页对齐。这些是练习范围外的扩展点。

---

## 8. 练习六：`simple_hv`

### 8.1 实验目的

实现一个最小 RISC-V 虚拟化监视器（Hypervisor）：

1. 从文件系统加载客体内核镜像；
2. 建立第二阶段（stage-2）地址翻译，把客户机物理地址映射到宿主机物理地址；
3. 通过 `sret` 进入 VS-mode 执行客体内核；
4. 捕获客体的 `ecall` VM Exit（虚拟机退出），解析 SBI（Supervisor Binary Interface，管理者二进制接口）调用；
5. 验证客体的关机请求并正常关闭 QEMU。

### 8.2 原理分析

#### 8.2.1 特权级与寄存器

RISC-V H（Hypervisor）扩展引入：

- **HS-mode**：Hypervisor-extended Supervisor mode，运行虚拟机监视器；
- **VS-mode**：Virtual Supervisor mode，运行客户机操作系统内核；
- **VU-mode**：Virtual User mode，运行客户机用户程序（本练习未使用）。

关键 CSR（Control and Status Register，控制与状态寄存器）：

| 寄存器 | 作用 |
| --- | --- |
| `hstatus` | `SPV=1` 表示 `sret` 进入 VS-mode；`SPVP=1` 表示目标特权级为 VS |
| `hgatp` | 第二阶段页表根地址与翻译模式（Sv39x4） |
| `hedeleg` / `hideleg` | 异常/中断委托（本练习使用默认值） |
| `vsatp` | 客户机第一阶段的页表（本练习清零，使用 Bare 模式） |

`sstatus` 中本练习涉及的位：SPP（Supervisor Previous Privilege，陷阱前的特权级）、SPIE（Supervisor Previous Interrupt Enable，陷阱前的中断使能）、SIE（Supervisor Interrupt Enable，管理程序中断使能）。`sret` 执行时会把 SPIE 复制到 SIE，因此清空 SPIE 可以让客户机在关闭中断的状态下运行。

#### 8.2.2 两阶段地址翻译

```text
VS-mode 访存地址（客户机虚拟地址 GA）
        │  vsatp（本练习为 Bare，GA == GPA）
        ▼
GPA（Guest Physical Address，客户机物理地址）0x8020_0000
        │  hgatp（Sv39x4 第二阶段页表）
        ▼
HPA（Host Physical Address，宿主机物理地址）0x8064_2000
```

`hgatp` 使用 Sv39x4 模式（模式字段值为 8）：根页表有 2048 项，且必须 **16 KiB 对齐**。本练习只使用 GPA 低 39 位以内地址，Sv39 与 Sv39x4 的页表索引在该范围内完全一致，因此可以把一个普通 Sv39 页表的 4 KiB 根内容复制到 16 KiB 对齐的新根中，其余 12 KiB 清零。

#### 8.2.3 进入与退出客户机

进入客户机由 `vcpu::_run_guest`（汇编 `guest.S`）完成：

1. 保存宿主 GPR（General Purpose Register，通用寄存器）与 `sstatus`、`hstatus`、`scounteren`、`sepc`、`stvec`、`sscratch`；
2. 装入客户机寄存器与 CSR；
3. 把 `stvec` 临时改为 `_guest_exit`，这样客体陷阱会直接回到退出处理代码；
4. 执行 `sret` 进入 VS-mode。

客体执行 `ecall` 时，硬件把 `scause` 置为 `VirtualSupervisorEnvCall`（值为 10），跳转到 `_guest_exit`。退出代码保存客户机状态、恢复宿主状态并返回 Rust 的 `vmexit_handler`。

#### 8.2.4 SBI 与关机

客体的 `ecall` 使用 legacy SBI 约定：

```text
a7 = 8（LEGACY_SHUTDOWN）
a0 / a1 = 任意标记值
```

监视器从保存的客户机寄存器 `a0..a7` 解析调用，确认是 `Reset(Shutdown)`，检查标记值，最后调用 `axhal::misc::terminate()`；后者通过 OpenSBI 的 SRST（System Reset，系统复位）扩展关机。

### 8.3 核心代码（逐行注释）

#### 8.3.1 客体内核

文件：`payload/skernel2/src/main.rs`

```rust
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    asm!(
        "li a0, 0x6688",   // 第一个标记，供 hypervisor 校验
        "li a1, 0x1234",   // 第二个标记
        "li a7, 8",        // SBI legacy 扩展号：关机
        "ecall",           // 陷入 HS-mode，产生 VirtualSupervisorEnvCall
        options(noreturn)
    )
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```

该二进制通过 `rust-objcopy -O binary` 转换为 20 字节的扁平镜像，加载到 GPA `0x8020_0000`。

#### 8.3.2 加载客体并建立第二阶段页表

文件：`exercises/simple_hv/src/main.rs`

```rust
const VM_ENTRY: usize = 0x8020_0000;   // 客体内核链接的 GPA

/// 把客体镜像复制到一个新分配的物理页，返回其 HPA
fn load_guest_image(image: &[u8]) -> PhysAddr {
    assert!(!image.is_empty() && image.len() <= PAGE_SIZE_4K);

    // 从全局分配器申请一页（返回内核直接映射的虚拟地址）
    let page = std::os::arceos::modules::axalloc::global_allocator()
        .alloc_pages(1, PAGE_SIZE_4K)
        .expect("no memory for the guest image");
    let vaddr = VirtAddr::from_usize(page);

    unsafe {
        core::ptr::write_bytes(vaddr.as_mut_ptr(), 0, PAGE_SIZE_4K); // 清零
        core::ptr::copy_nonoverlapping(image.as_ptr(), vaddr.as_mut_ptr(), image.len());
    }
    virt_to_phys(vaddr)   // 直接映射：VA -> PA
}

/// 建立 GPA(VM_ENTRY) -> HPA(guest_paddr) 的 Sv39 页表，
/// 并把根复制到 16 KiB 对齐的 Sv39x4 根页表
fn build_stage2_page_table(guest_paddr: PhysAddr) -> PhysAddr {
    let mut pt = PageTable::try_new().expect("failed to allocate stage-2 root");

    // 映射一个 4 KiB 页，权限为 R/W/X/U（G-stage 中 U 位不参与权限判定）
    let flush = pt.map(
        VirtAddr::from_usize(VM_ENTRY),
        guest_paddr,
        PageSize::Size4K,
        MappingFlags::READ | MappingFlags::WRITE
            | MappingFlags::EXECUTE | MappingFlags::USER,
    ).expect("failed to map the guest page");
    flush.ignore();

    // hgatp 的 Sv39x4 根必须 16 KiB 对齐，且有 2048 项
    let aligned = std::os::arceos::modules::axalloc::global_allocator()
        .alloc_pages(4, 0x4000)
        .expect("no memory for the Sv39x4 root");
    let root_vaddr = VirtAddr::from_usize(aligned);
    unsafe {
        core::ptr::write_bytes(root_vaddr.as_mut_ptr(), 0, 0x4000);
        core::ptr::copy_nonoverlapping(
            phys_to_virt(pt.root_paddr()).as_ptr(),  // 原 Sv39 根
            root_vaddr.as_mut_ptr(),                 // 16 KiB 对齐新根
            PAGE_SIZE_4K,
        );
    }
    let root_paddr = virt_to_phys(root_vaddr);

    // 泄漏临时页表：其下级页表仍被复制后的根引用
    forget(pt);
    root_paddr
}

/// 写入 hgatp：模式 8 = Sv39x4，根 PPN = 物理地址 >> 12
fn prepare_vm_pgtable(ept_root: PhysAddr) {
    let hgatp = 8usize << 60 | usize::from(ept_root) >> 12;
    unsafe {
        core::arch::asm!(
            "csrw hgatp, {hgatp}",
            hgatp = in(reg) hgatp,
        );
        core::arch::riscv64::hfence_gvma_all();  // 刷新 G-stage TLB
    }
}
```

#### 8.3.3 主流程与 VM Exit 处理

```rust
fn main() {
    ax_println!("Hypervisor ...");

    // 1. 从 FAT 文件系统读取 /sbin/skernel2
    let image = load_vm_image("/sbin/skernel2")
        .unwrap_or_else(|e| panic!("Cannot load guest image! {:?}", e));

    // 2. 复制到物理页并建立第二阶段映射
    let guest_paddr = load_guest_image(&image);
    let ept_root = build_stage2_page_table(guest_paddr);
    prepare_vm_pgtable(ept_root);

    // 3. 准备 hstatus/sepc/sstatus，进入 VS-mode
    let mut ctx = VmCpuRegisters::default();
    prepare_guest_context(&mut ctx);

    // 4. 关闭宿主中断，避免定时器中断被误认为 VM Exit
    unsafe { riscv::register::sstatus::clear_sie() };
    while !run_guest(&mut ctx) {}

    ax_println!("[SimpleHV]: ok!");
    axhal::misc::terminate();   // 通过 SBI SRST 关机
}

fn vmexit_handler(ctx: &mut VmCpuRegisters) -> bool {
    use scause::{Exception, Trap};
    match scause::read().cause() {
        Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
            let sbi_msg = SbiMessage::from_regs(ctx.guest_regs.gprs.a_regs()).ok();
            ax_println!("VmExit Reason: VSuperEcall: {:?}", sbi_msg);
            match sbi_msg {
                Some(SbiMessage::Reset(_)) => {
                    let a0 = ctx.guest_regs.gprs.reg(A0);
                    let a1 = ctx.guest_regs.gprs.reg(A1);
                    ax_println!("a0 = {:#x}, a1 = {:#x}", a0, a1);
                    assert_eq!(a0, 0x6688);   // 校验客体标记
                    assert_eq!(a1, 0x1234);
                    ax_println!("Shutdown vm normally!");
                    true                       // 通知主循环退出
                }
                Some(other) => panic!("unsupported SBI message: {:?}", other),
                None => panic!("bad SBI message!"),
            }
        }
        other => panic!("Unhandled trap: {:?}", other),
    }
}

fn prepare_guest_context(ctx: &mut VmCpuRegisters) {
    // hstatus.SPV=1：sret 进入客户机；hstatus.SPVP=1：VS-mode
    let mut hstatus = LocalRegisterCopy::<usize, hstatus::Register>::new(
        riscv::register::hstatus::read().bits(),
    );
    hstatus.modify(hstatus::spv::Guest);
    hstatus.modify(hstatus::spvp::Supervisor);
    CSR.hstatus.write_value(hstatus.get());
    ctx.guest_regs.hstatus = hstatus.get();

    // 客户机 sstatus：SPP=Supervisor；SPIE=0，使 sret 后 SIE=0
    let mut sstatus = sstatus::read();
    sstatus.set_spp(sstatus::SPP::Supervisor);
    sstatus.set_spie(false);
    ctx.guest_regs.sstatus = sstatus.bits();

    ctx.guest_regs.sepc = VM_ENTRY;   // 从 GPA 0x8020_0000 开始执行
}
```

#### 8.3.4 汇编进入/退出

`exercises/simple_hv/src/guest.S` 的核心逻辑：

```asm
.global _run_guest
_run_guest:
    /* 保存宿主 ra/gp/tp/s0-s11/a1-a7/sp 等寄存器 */
    /* 交换 sstatus、hstatus、scounteren、sepc、stvec、sscratch */
    la    t1, _guest_exit
    csrrw t1, stvec, t1        # 客体陷阱直接回到 _guest_exit
    /* 装入客户机 GPR 与 CSR */
    sret                        # 进入 VS-mode

_guest_exit:
    /* 保存客户机 GPR，恢复宿主 CSR 与 GPR，返回 Rust */
```

`vcpu.rs` 使用 `memoffset::offset_of!` 计算 `VmCpuRegisters` 各字段偏移，并通过 `global_asm!` 把这些偏移作为常量传给汇编，实现 C ABI（Application Binary Interface，应用二进制接口）的结构体与汇编寄存器交换。

#### 8.3.5 SBI 解码

文件：`exercises/simple_hv/src/sbi/mod.rs`（仓库已有实现，直接复用）

```rust
pub fn from_regs(args: &[usize]) -> AxResult<Self> {
    match args[7] {                          // a7 为 SBI 扩展号
        sbi_spec::legacy::LEGACY_SHUTDOWN => // 8
            Ok(SbiMessage::Reset(ResetFunction::shutdown())),
        sbi_spec::srst::EID_SRST =>
            ResetFunction::from_regs(args).map(SbiMessage::Reset),
        // base / putchar / getchar / set_timer / rfnc / pmu ...
        _ => Err(AxError::NotFound),
    }
}
```

### 8.4 改动位置

| 文件 | 改动 |
| --- | --- |
| `exercises/simple_hv/src/main.rs` | 重写加载、第二阶段页表、进入/退出与关机流程 |
| `exercises/simple_hv/src/loader.rs` | 改为一次性读取完整客体镜像 |
| `payload/skernel2/src/main.rs` | 客体内核改为设置标记并发起 SBI 关机 |
| `scripts/make/qemu.mk` | 增加 `QEMU_CPU` 变量，便于启用 H 扩展 |
| `exercises/simple_hv/scripts/test-simple_hv.sh` | 测试脚本 |
| `exercises/scripts/test-simple_hv.sh` | 统一入口包装脚本 |

`vcpu.rs`、`regs.rs`、`csrs.rs`、`guest.S` 与 `sbi/` 子模块沿用模板实现；本练习的主要工作是正确配置和使用它们。

### 8.5 编译与测试步骤

```bash
cd /path/to/arceos

# 1. 构建客体（生成 ELF 后转为扁平二进制）
cargo build -p skernel2 --target riscv64gc-unknown-none-elf --release
rust-objcopy --binary-architecture=riscv64 --strip-all -O binary \
    target/riscv64gc-unknown-none-elf/release/skernel2 \
    payload/skernel2/skernel2

# 2. 生成包含 /sbin/skernel2 的 FAT32 镜像
python3 exercises/tools/mkdisk.py exercises/simple_hv/disk.img \
        /sbin/skernel2="$PWD/payload/skernel2/skernel2"

# 3. 启动 hypervisor（QEMU_CPU=max 确保 H 扩展存在）
make A=exercises/simple_hv BLK=y DISK_IMG="$PWD/exercises/simple_hv/disk.img" \
     QEMU_CPU=max LOG=warn QEMU_LOG=n PFLASH=n run

# 4. 测试脚本
cd exercises
./scripts/test-simple_hv.sh
```

### 8.6 实验现象

```text
Hypervisor ...
Load guest image: /sbin/skernel2
Guest image size: 20 byte(s)
Guest: GPA 0x80200000 -> HPA PA:0x80642000, 20 bytes
Stage-2 root: PA:0x80649000
VmExit Reason: VSuperEcall: Some(Reset(Reset { reset_type: Shutdown, reason: NoReason }))
a0 = 0x6688, a1 = 0x1234
Shutdown vm normally!
[SimpleHV]: ok!
```

测试脚本结果：

```text
PASS: found hypervisor start
PASS: found guest image loaded
PASS: found stage-2 mapping
PASS: found VM exit captured
PASS: found guest marker registers
PASS: found normal guest shutdown
PASS: found hypervisor success marker
test-simple_hv.sh: ALL CHECKS PASSED
```

### 8.7 结果分析

- 客体镜像只有 20 字节，被复制到 HPA `0x8064_2000` 的一个 4 KiB 页中；第二阶段页表把 GPA `0x8020_0000` 映射到该页。
- `Stage-2 root: 0x8064_9000` 是 `0x4000` 的整数倍，满足 Sv39x4 根页表的 16 KiB 对齐要求。
- `sret` 成功进入 VS-mode，客体执行 `li` 与 `ecall`；`ecall` 产生 `VirtualSupervisorEnvCall`。
- `SbiMessage::from_regs` 从 `a7=8` 识别出 `LEGACY_SHUTDOWN`，`ResetFunction::shutdown()` 的调试输出为 `Reset { reset_type: Shutdown, reason: NoReason }`。
- 客体 `a0=0x6688`、`a1=0x1234` 与预期一致，说明寄存器在进入/退出过程中被正确保存和恢复。
- 监视器打印 `Shutdown vm normally!` 后调用 `axhal::misc::terminate()`，OpenSBI 执行 SRST 关机，QEMU 退出码为 0。

### 8.8 踩坑总结

1. **`riscv::interrupt::disable()` 在 S-mode 非法**：该旧版 `riscv` crate 的实现写 `mstatus`，而 `mstatus` 是 M-mode CSR。首次运行在 `csrc mstatus, a0` 处触发 `IllegalInstruction`。改为 `riscv::register::sstatus::clear_sie()` 清除 `sstatus.SIE`。
2. **`hgatp` 根页表必须 16 KiB 对齐**：`PageTable::try_new()` 只分配 4 KiB 页，且不保证 16 KiB 对齐；直接把它的地址写入 `hgatp` 在部分 QEMU 实现上会导致客户机页表遍历错误。改为申请 4 个连续页（对齐 `0x4000`）、清零 16 KiB、复制 Sv39 根的 4 KiB 内容。
3. **Sv39 与 Sv39x4 的索引差异**：Sv39x4 根有 2048 项，但 GPA 低于 2^39 时根索引高位为 0，取低 9 位与 Sv39 相同；因此复制 512 项即可覆盖本练习的地址。
4. **原模板 loader 有越界读取**：`let mut buf = [0u8; 64]; ... copy_nonoverlapping(buf.as_ptr(), ..., PAGE_SIZE_4K)` 会从 64 字节栈数组复制 4 KiB。改为 `read_to_end` 读取完整文件后再复制。
5. **客体的 `mhartid` 访问**：原 `skernel2` 在 VS-mode 执行 `csrr a1, mhartid`，而 M-mode CSR 在 VS-mode 不可访问，会触发非法指令。改为直接设置 `a0/a1` 标记并发起关机。
6. **重复的 `sbi` 模块**：目录中已存在 `src/sbi/`，最初又新建 `src/sbi.rs`，Rust 报 `E0761`（同名模块不明确）。删除重复文件并复用已有 SBI 解码器。
7. **定时器中断被误判为 VM Exit**：`_run_guest` 期间 `stvec` 指向 `_guest_exit`，若宿主定时器中断到来会进入退出路径。进入客户机前关闭 `sstatus.SIE`，并让客户机 `SPIE=0`，避免此类干扰。
8. **QEMU CPU 的 H 扩展**：本环境 QEMU 10.2 的默认 `rv64` CPU 已报告 `rv64imafdch`，但为可移植性新增 `QEMU_CPU` 变量并使用 `-cpu max`，确保 H 扩展存在。
9. **页表泄漏是有意为之**：临时 `PageTable` 的下级页表仍被复制后的 Sv39x4 根引用，因此调用 `mem::forget` 防止 Drop 释放；监视器生命周期内可接受。

---

## 9. 整体总结

### 9.1 按操作系统模块归纳

| 模块 | 练习 | 关键收获 |
| --- | --- | --- |
| I/O（Input/Output，输入/输出） | `print_with_color`、`sys_map` | 串口字节流与 ANSI 控制序列；用户态 `write` 经文件描述符表到达控制台 |
| 文件系统 | `ramfs_rename`、`sys_map` | VFS 节点抽象、挂载点路由、RAMFS 目录项移动；FAT32 镜像布局与 ELF 文件加载 |
| 内存分配 | `alt_alloc`、`sys_map`、`simple_hv` | 全局分配器 trait 体系与替换；用户地址空间按需分配；第二阶段页表与物理页管理 |
| 容器 | `support_hashmap` | `no_std` 下的哈希表接入与默认哈希器；`axstd` 标准库兼容层设计 |
| 系统调用 | `sys_map` | `ecall` 陷入、`TrapFrame` 参数传递、`linkme` 分发、`mmap` 权限与地址管理 |
| 虚拟化 | `simple_hv` | H 扩展特权级、`hstatus`/`hgatp`、两阶段翻译、VM Exit 与 SBI 调用 |

### 9.2 跨模块设计经验

1. **`no_std` 不是“没有生态”**：通过 `alloc`、`hashbrown`、`axstd` 的兼容层，可以在没有 Rust 标准库的内核中复用大量成熟组件；关键是正确处理全局分配器、默认哈希器与特性门控。
2. **VFS 抽象的价值**：RAMFS 只需要实现目录项移动，就自动获得 `File::create`、`fs::rename` 等上层接口；挂载点路由则把“路径属于哪个文件系统”与“文件系统如何操作”解耦。
3. **地址空间是系统调用的核心资源**：`mmap` 的实现不只是“找一段地址”，还涉及 ELF 加载、页表填充、物理页归属、权限与 TLB 刷新；任何一步顺序错误都会表现为用户态页错误。
4. **虚拟化是地址翻译与控制流状态的组合**：`hgatp` 负责“地址从哪里来”，`hstatus`/`sepc`/`sstatus` 负责“执行流去哪里”，`stvec` 与 `_guest_exit` 负责“退出后回到哪里”；三者缺一不可。
5. **测试脚本应当验证行为而不是日志格式**：内核日志会交织、会带颜色、会随版本变化；脚本使用关键行为标记（如 `Bump tests run OK!`、`copied 15 byte(s)`、`a0 = 0x6688`）进行断言，并用超时与退出码兜底。
6. **可重复的测试基础设施很重要**：自定义 FAT32 镜像生成器与无 libc 用户态 payload，使练习不依赖缺失的 `mtools` 与 `riscv64-linux-musl-gcc`，在任何具备 Rust 工具链和 QEMU 的环境中都能复现。

### 9.3 最终测试结果

从 `exercises/` 目录依次执行：

```bash
./scripts/test-print.sh
./scripts/test-ramfs_rename.sh
./scripts/test-alt_alloc.sh
./scripts/test-support_hashmap.sh
./scripts/test-sys_map.sh
./scripts/test-simple_hv.sh
```

结果：

```text
print: PASS
ramfs_rename: PASS
alt_alloc: PASS
support_hashmap: PASS
sys_map: PASS
simple_hv: PASS
```

课程提供的统一评分脚本位于仓库上一级：

```bash
cd /path/to/2026a-arceos-345gug
./scripts/total-test.sh
```

其输出依次包含 `print_with_color pass`、`ramfs_rename pass`、`alt_alloc pass`、`support_hashmap pass`、`sys_mmap pass`、`simple_hv pass`，最后一行是总分：

```text
600
```

六个练习全部通过（每个 100 分），QEMU 均在程序结束时正常退出。实验完整覆盖了 ArceOS 的控制台 I/O、文件系统、内存分配、容器、系统调用与 RISC-V 虚拟化六个核心模块。
