//! 网络层错误类型；错误文本一律英文（见 CODEBUDDY.md 第 3 节）。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    /// 底层传输失败。
    #[error("transport error: {0}")]
    Transport(String),
    /// 本机 IO 失败。
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// 包格式非法（长度、magic、校验和、字段越界）。
    #[error("malformed packet: {0}")]
    Malformed(String),
    /// 协议版本不匹配，明确拒绝而非兼容。
    #[error("protocol version mismatch: expected={expected} found={found}")]
    VersionMismatch { expected: u8, found: u8 },
    /// 命令帧已过期，无法提交。
    #[error("frame in past: frame={frame} current={current}")]
    FrameInPast { frame: u64, current: u64 },
    /// 逐帧校验不一致，显式上报。
    #[error("desync detected: frame={frame} local={local:#x} remote={remote:#x} field={field:?}")]
    Desync {
        frame: u64,
        local: u64,
        remote: u64,
        field: Option<&'static str>,
    },
    /// 会话 / 身份 / 游标错误。
    #[error("session error: {0}")]
    Session(String),
}
