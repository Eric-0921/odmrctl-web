//! OE1022D 第一版命令 helper。
//!
//! 设计原则：
//! - 只保留第一版需要的固定配置命令和 `RALL?`
//! - 命令名尽量贴手册
//! - 不在这里发明更高层的通道配置抽象

/// `FMODD i,j`：设置参考源。
pub fn oe1022d_set_reference_source(channel: u8, source: u8) -> String {
    format!("FMODD {channel},{source}")
}

/// `FMODD? i`：查询参考源。
pub fn oe1022d_query_reference_source(channel: u8) -> String {
    format!("FMODD? {channel}")
}

/// `FREQD i,f`：设置内部参考频率，单位 Hz。
pub fn oe1022d_set_reference_frequency_hz(channel: u8, hz: f64) -> String {
    format!("FREQD {channel},{hz}")
}

/// `ISRCD i,j`：设置输入方式。
pub fn oe1022d_set_input_source(channel: u8, source: u8) -> String {
    format!("ISRCD {channel},{source}")
}

/// `IGNDD i,j`：设置输入接地方式。
pub fn oe1022d_set_input_grounding(channel: u8, grounding: u8) -> String {
    format!("IGNDD {channel},{grounding}")
}

/// `ICPLD i,j`：设置输入耦合方式。
pub fn oe1022d_set_input_coupling(channel: u8, coupling: u8) -> String {
    format!("ICPLD {channel},{coupling}")
}

/// `ILIND i,j`：设置陷波器模式。
pub fn oe1022d_set_line_notch_filter(channel: u8, filter: u8) -> String {
    format!("ILIND {channel},{filter}")
}

/// `RMODD i,j`：设置动态储备模式。
pub fn oe1022d_set_dynamic_reserve(channel: u8, mode: u8) -> String {
    format!("RMODD {channel},{mode}")
}

/// `SENSD i,j`：设置灵敏度索引。
pub fn oe1022d_set_sensitivity_index(channel: u8, index: u8) -> String {
    format!("SENSD {channel},{index}")
}

/// `OFLTD i,j`：设置时间常数索引。
pub fn oe1022d_set_time_constant_index(channel: u8, index: u8) -> String {
    format!("OFLTD {channel},{index}")
}

/// `OFSLD i,j`：设置滤波器斜率索引。
pub fn oe1022d_set_filter_slope(channel: u8, slope: u8) -> String {
    format!("OFSLD {channel},{slope}")
}

/// `SYNCD i,j`：设置同步滤波器开关。
pub fn oe1022d_set_sync_filter(channel: u8, enabled: u8) -> String {
    format!("SYNCD {channel},{enabled}")
}

/// `RALL?`：读取一帧全局测量和配置二进制数据。
pub fn oe1022d_rall_query() -> &'static str {
    "RALL?"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_helpers_keep_manual_shape() {
        assert_eq!(oe1022d_set_reference_source(2, 0), "FMODD 2,0");
        assert_eq!(oe1022d_query_reference_source(1), "FMODD? 1");
        assert_eq!(
            oe1022d_set_reference_frequency_hz(1, 2048.0),
            "FREQD 1,2048"
        );
    }

    #[test]
    fn input_and_filter_helpers_keep_manual_shape() {
        assert_eq!(oe1022d_set_input_source(2, 0), "ISRCD 2,0");
        assert_eq!(oe1022d_set_input_grounding(2, 0), "IGNDD 2,0");
        assert_eq!(oe1022d_set_input_coupling(2, 1), "ICPLD 2,1");
        assert_eq!(oe1022d_set_line_notch_filter(2, 0), "ILIND 2,0");
        assert_eq!(oe1022d_set_dynamic_reserve(2, 1), "RMODD 2,1");
        assert_eq!(oe1022d_set_sensitivity_index(2, 24), "SENSD 2,24");
        assert_eq!(oe1022d_set_time_constant_index(2, 9), "OFLTD 2,9");
        assert_eq!(oe1022d_set_filter_slope(2, 1), "OFSLD 2,1");
        assert_eq!(oe1022d_set_sync_filter(2, 0), "SYNCD 2,0");
    }

    #[test]
    fn rall_query_is_exact() {
        assert_eq!(oe1022d_rall_query(), "RALL?");
    }
}
