//! Offline GLSL ES compilation for PowerVR SGX543 revision 216.
//!
//! The native compiler is built on Linux and requires a GNU-compatible C
//! toolchain. It does not require EGL, a GPU or the Vita SDK. On other targets,
//! [`AVAILABLE`] is false and [`Compiler::new`] returns an error.
//!
//! [`Compiler::compile`] returns diagnostics and the generated USP program size.
//! [`Compiler::compile_binary`] also returns a complete `GL_SGX_BINARY_IMG`
//! container for a matching Vita driver. Neither operation links shader pairs
//! or executes GPU instructions.
//!
//! ```
//! use pvr_compiler::{AVAILABLE, Compiler, Stage};
//!
//! # fn main() -> Result<(), String> {
//! if AVAILABLE {
//!     let mut compiler = Compiler::new()?;
//!     let result = compiler.compile_binary(
//!         Stage::Vertex,
//!         "void main() { gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }",
//!     )?;
//!     if result.success {
//!         assert!(!result.binary.is_empty());
//!     } else {
//!         eprintln!("{}", result.log);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
#[cfg(pvr_compiler_available)]
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    ptr::NonNull,
    sync::Mutex,
};

/// Whether this build includes the native compiler.
pub const AVAILABLE: bool = cfg!(pvr_compiler_available);

/// Shader stage passed to the GLSL compiler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Vertex shader.
    Vertex,
    /// Fragment shader.
    Fragment,
}

/// Compilation status, diagnostics and optional serialized shader data.
#[must_use]
#[derive(Clone, Debug)]
pub struct Compilation {
    /// False when the compiler rejects the source; inspect [`Self::log`].
    pub success: bool,
    /// Original compiler diagnostics, including warnings on successful builds.
    pub log: String,
    /// USP program size for [`Compiler::compile`], or complete container size
    /// for [`Compiler::compile_binary`]. Only meaningful on success.
    pub binary_bytes: u32,
    /// Complete `GL_SGX_BINARY_IMG` container. Empty for [`Compiler::compile`]
    /// and for rejected source.
    pub binary: Vec<u8>,
}

// Parser globals require serialization even across separate compiler contexts.
#[cfg(pvr_compiler_available)]
static COMPILER: Mutex<()> = Mutex::new(());

/// Owns a native compiler context; no graphics context is required.
///
/// Reuse an instance for an ordered sequence of compilations. Built-in symbol
/// state can affect warnings on later calls. All instances share an internal
/// lock because the native parser uses process-global state. Native instances
/// cannot be sent or shared across threads.
pub struct Compiler {
    #[cfg(pvr_compiler_available)]
    context: NonNull<c_void>,
}

#[allow(unsafe_code)]
#[cfg(pvr_compiler_available)]
unsafe extern "C" {
    fn pvr_compiler_create() -> *mut c_void;
    fn pvr_compiler_compile(
        context: *mut c_void,
        source: *const c_char,
        fragment: c_int,
        log: *mut *mut c_char,
        binary_bytes: *mut u32,
        binary: *mut *mut c_void,
    ) -> c_int;
    fn pvr_compiler_free_log(log: *mut c_char);
    fn pvr_compiler_destroy(context: *mut c_void);
}

impl Compiler {
    /// Creates a compiler context.
    ///
    /// # Errors
    /// Returns an error if this target is unsupported or initialization fails.
    #[allow(unsafe_code)]
    pub fn new() -> Result<Self, String> {
        #[cfg(pvr_compiler_available)]
        {
            let _guard = COMPILER.lock().unwrap_or_else(|e| e.into_inner());
            let context = NonNull::new(unsafe { pvr_compiler_create() })
                .ok_or("PVR compiler initialization failed")?;
            Ok(Self { context })
        }
        #[cfg(not(pvr_compiler_available))]
        {
            Err("PVR compiler is only available on Linux targets".into())
        }
    }

    /// Compiles source and returns diagnostics and the USP program size.
    ///
    /// The returned [`Compilation::binary`] is empty. Invalid GLSL returns
    /// `Ok` with [`Compilation::success`] set to false.
    ///
    /// # Errors
    /// Returns an error for embedded NUL bytes or a native adapter failure.
    pub fn compile(&mut self, stage: Stage, source: &str) -> Result<Compilation, String> {
        self.compile_inner(stage, source, false)
    }

    /// Compiles source into a complete `GL_SGX_BINARY_IMG` container.
    ///
    /// The container targets SGX543 revision 216, driver build 869593 and GLSL
    /// compiled interface version 2. Loading still requires a matching driver;
    /// compilation does not validate shader linking or rendering.
    /// Invalid GLSL returns `Ok` with [`Compilation::success`] set to false.
    ///
    /// # Errors
    /// Returns an error for embedded NUL bytes, a native adapter failure or a
    /// serialization failure.
    pub fn compile_binary(&mut self, stage: Stage, source: &str) -> Result<Compilation, String> {
        self.compile_inner(stage, source, true)
    }
}

#[allow(unsafe_code)]
#[cfg(pvr_compiler_available)]
impl Compiler {
    fn compile_inner(
        &mut self,
        stage: Stage,
        source: &str,
        packed: bool,
    ) -> Result<Compilation, String> {
        let source = CString::new(source).map_err(|_| "shader contains a NUL byte")?;
        let _guard = COMPILER.lock().unwrap_or_else(|e| e.into_inner());
        let mut log = std::ptr::null_mut();
        let mut binary_bytes = 0;
        let mut binary = std::ptr::null_mut();
        // C copies the log before freeing the compiled program. Always release
        // that copy using the same C allocator, including failed compilations.
        let result = unsafe {
            pvr_compiler_compile(
                self.context.as_ptr(),
                source.as_ptr(),
                i32::from(stage == Stage::Fragment),
                &mut log,
                &mut binary_bytes,
                if packed {
                    &mut binary
                } else {
                    std::ptr::null_mut()
                },
            )
        };
        let data = if binary.is_null() {
            Vec::new()
        } else {
            let data =
                unsafe { std::slice::from_raw_parts(binary.cast::<u8>(), binary_bytes as usize) }
                    .to_vec();
            unsafe { pvr_compiler_free_log(binary.cast()) };
            data
        };
        if log.is_null() {
            return Err("PVR compiler could not allocate its log".into());
        }
        let text = unsafe { CStr::from_ptr(log).to_string_lossy().into_owned() };
        unsafe { pvr_compiler_free_log(log) };
        if result < 0 {
            Err(text)
        } else {
            Ok(Compilation {
                success: result != 0,
                log: text,
                binary_bytes,
                binary: data,
            })
        }
    }
}

#[allow(unsafe_code)]
#[cfg(pvr_compiler_available)]
impl Drop for Compiler {
    fn drop(&mut self) {
        let _guard = COMPILER.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { pvr_compiler_destroy(self.context.as_ptr()) };
    }
}

#[cfg(not(pvr_compiler_available))]
impl Compiler {
    fn compile_inner(
        &mut self,
        _stage: Stage,
        _source: &str,
        _packed: bool,
    ) -> Result<Compilation, String> {
        Err("PVR compiler is only available on Linux targets".into())
    }
}
