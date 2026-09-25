# pvr-compiler

面向 PowerVR SGX543 的离线 GLSL ES 编译器。通过 Rust API 调用 PVR 的 GLSL 前端与 USC 后端，可检查着色器、获取编译诊断，并生成供匹配的 Vita 驱动加载的二进制着色器。

编译过程在主机 CPU 上运行，无需 GPU、EGL、VitaSDK 或 ARM 工具链。

## 平台与构建要求

| 项目 | 要求 |
| --- | --- |
| Rust | 1.95 或更新版本，edition 2024 |
| 原生编译器 | Linux，GNU 兼容 C 工具链及系统数学库 |
| 其他目标平台 | Rust API 可构建，`AVAILABLE` 为 `false`，`Compiler::new()` 返回错误 |
| 输出目标 | SGX543MP4 revision 216 |

在 crate 根目录运行：

```sh
cargo build --release
cargo test --release
cargo clippy --all-targets -- -D warnings
```

Linux 构建会编译随 crate 提供的 C 源码，首次构建时间较长。构建不下载驱动源码，也不读取 crate 目录之外的驱动源文件。非 Linux 平台的测试只覆盖不可用状态及可编译的 Rust 接口，不执行原生编译器回归测试。

## 使用

从 GitHub 添加依赖：

```toml
[dependencies]
pvr-compiler = { git = "https://github.com/jhq223/pvr-compiler.git" }
```

```rust
use pvr_compiler::{Compiler, Stage};

fn main() -> Result<(), String> {
    let mut compiler = Compiler::new()?;
    let result = compiler.compile_binary(
        Stage::Fragment,
        "precision mediump float; void main() { gl_FragColor = vec4(1.0); }",
    )?;

    if !result.success {
        return Err(result.log);
    }
    if !result.log.is_empty() {
        eprintln!("{}", result.log);
    }
    std::fs::write("fragment.sgx", &result.binary).map_err(|error| error.to_string())?;
    Ok(())
}
```

### 返回值与错误

| API / 字段 | 含义 |
| --- | --- |
| `Compiler::new()` | 创建独立的编译上下文，释放由 `Drop` 完成 |
| `compile(stage, source)` | 返回编译状态、原始日志和 USP 程序大小；`binary` 为空 |
| `compile_binary(stage, source)` | 额外返回完整的 `GL_SGX_BINARY_IMG` 容器 |
| `Compilation::success` | 编译是否成功；语法或类型错误会得到 `Ok`，但此字段为 `false` |
| `Compilation::log` | 编译器诊断；成功时也可能包含警告 |
| `Compilation::binary_bytes` | 成功时的 USP 程序大小，或 `compile_binary` 的完整容器大小 |
| `Err(String)` | 平台不受支持、初始化失败、源码包含 NUL，或原生适配／序列化失败 |

同一序列的着色器应复用一个 `Compiler`。编译器保留部分内建符号状态，后续调用的警告可能与首次调用不同。原生上下文不实现 `Send` / `Sync`；不同实例也通过内部全局锁串行调用原生编译器。

### 二进制兼容性

生成的容器包含校验值、符号表和版本信息，目标为 SGX543 revision 216、驱动 build 869593、GLSL compiled interface version 2。应用可将其交给匹配驱动的 `glShaderBinary`，并应检查加载结果。

该 API 编译单个着色器，不链接顶点／片元着色器组合，也不执行 SGX 指令。离线编译成功不代表程序链接或实际渲染一定成功。

## 源码布局

| 路径 | 内容 |
| --- | --- |
| `src/lib.rs` | Rust API、上下文所有权和调用串行化 |
| `bridge.c`、`host.h` | 主机适配、编译参数和二进制封装入口 |
| `build.rs` | C 源文件选择、编译宏和构建配置 |
| `vendor/` | PVR GLSL、USC、二进制封装器及所需头文件、生成的解析器 |
| `tests/` | 自包含的编译、诊断、容器格式和平台支持回归测试 |

编译器源码取自 PVR_PSP2 驱动的编译路径，保留原有版权声明。主机适配包含指针宽度与格式处理，并禁用 strict aliasing 以兼容旧 USC 的位模式转换；SGX543 奇数编号向量硬件常量的搬运修复也包含在此版本中。
