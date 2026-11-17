//! Yali 内核骨架（M-1 阶段占位，T-002/T-003 填充真实 Schema 与管道）
//!
//! 设计权威：docs/DESIGN.md

pub mod profile;

pub use profile::Profile;

/// 内核版本号，与 CLI `yali --version` 联动
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_version_is_semver() {
        let v = CORE_VERSION;
        assert!(v.matches('.').count() >= 2, "not semver: {v}");
    }
}
