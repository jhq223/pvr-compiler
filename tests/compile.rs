#![cfg(target_os = "linux")]
use pvr_compiler::{Compiler, Stage};

#[test]
fn serialized_shader_has_vita_driver_revision_and_complete_payload() {
    let mut compiler = Compiler::new().unwrap();
    for (stage, source) in [
        (Stage::Vertex, include_str!("fixtures/constructor.vert")),
        (
            Stage::Fragment,
            "precision mediump float; void main() { gl_FragColor = vec4(1.0); }",
        ),
    ] {
        let result = compiler.compile_binary(stage, source).unwrap();
        assert!(result.success, "{}", result.log);
        let bytes = &result.binary;
        assert_eq!(result.binary_bytes as usize, bytes.len());
        let u32_at = |at| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!(
            u32_at(0),
            0x38B4FA10,
            "SGX container, not a bare USP program"
        );
        assert_eq!(&bytes[8..16], &[0, 1, 5, 0x43, 2, 0x16, 0, 0]);
        assert_eq!(u32_at(16), 869593, "matching 1.8 Vita driver build");
        assert_eq!(u32_at(20), 2, "GLSL compiled interface version");
        assert_eq!(u32_at(28) as usize + 32, bytes.len());
        assert!(bytes.len() > 128);
    }
    let invalid = compiler
        .compile_binary(
            Stage::Fragment,
            "precision mediump float; void main() { gl_FragColor = missing_variable; }",
        )
        .unwrap();
    assert!(!invalid.success);
    assert!(invalid.binary.is_empty());
}

#[test]
fn native_compiler_accepts_both_stages_and_rejects_invalid_glsl() {
    let mut compiler = Compiler::new().unwrap();
    for (stage, source) in [
        (
            Stage::Vertex,
            "attribute vec2 a_unit; void main() { gl_Position = vec4(a_unit, 0.0, 1.0); }",
        ),
        (
            Stage::Fragment,
            "precision mediump float; void main() { gl_FragColor = vec4(0.2, 0.6, 0.8, 1.0); }",
        ),
    ] {
        let result = compiler.compile(stage, source).unwrap();
        assert!(result.success, "{}", result.log);
        assert!(result.binary_bytes > 12, "USC must produce a USP shader");
    }
    let invalid = compiler
        .compile(
            Stage::Fragment,
            "precision mediump float; void main() { gl_FragColor = missing_variable; }",
        )
        .unwrap();
    assert!(!invalid.success);
    assert!(invalid.log.contains("missing_variable"), "{}", invalid.log);
}

#[test]
fn repeated_constructor_compilation_preserves_native_warning_behavior() {
    let mut compiler = Compiler::new().unwrap();
    let source = include_str!("fixtures/constructor.vert");
    let first = compiler.compile(Stage::Vertex, source).unwrap();
    assert!(first.success, "{}", first.log);
    assert!(!first.log.contains("WARNING:"), "{}", first.log);
    let result = compiler.compile(Stage::Vertex, source).unwrap();
    assert!(result.success, "{}", result.log);
    assert!(
        result.log.contains(
            "0:11: 'function_call_constructor@vec4_vec4@17' : used without being initialised"
        ),
        "{}",
        result.log
    );
    assert!(result.binary_bytes > 12);
}

#[test]
fn warnings_are_preserved_and_later_compilations_still_work() {
    let mut compiler = Compiler::new().unwrap();
    let warning = compiler
        .compile(
            Stage::Fragment,
            "precision mediump float; void main() { float value; gl_FragColor = vec4(value); }",
        )
        .unwrap();
    assert!(warning.success, "{}", warning.log);
    assert!(
        warning.log.contains("without being initialised"),
        "{}",
        warning.log
    );
    assert!(
        compiler
            .compile(Stage::Vertex, "void main() { gl_Position = vec4(1.0); }")
            .unwrap()
            .success
    );
    assert!(compiler.compile(Stage::Vertex, "void\0main").is_err());
}
