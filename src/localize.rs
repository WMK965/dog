/// Language detection and string localization.
///
/// Automatically detects Chinese vs English based on the system locale
/// and provides translated strings for all user-facing messages.

use std::sync::atomic::{AtomicBool, Ordering};

static IS_CHINESE: AtomicBool = AtomicBool::new(false);

/// Call once at startup to detect the system language.
pub fn detect() {
    let zh = detect_chinese_locale();
    IS_CHINESE.store(zh, Ordering::Relaxed);
}

/// Returns true if the detected language is Chinese.
fn is_chinese() -> bool {
    IS_CHINESE.load(Ordering::Relaxed)
}

#[allow(unsafe_code)]
fn detect_chinese_locale() -> bool {
    #[cfg(windows)]
    {
        // GetUserDefaultUILanguage returns LANGID, e.g. 0x0804 for zh-CN
        let lang_id = unsafe { windows_lang_id() };
        let primary = lang_id & 0x00FF;
        // Chinese: 0x04 (zh-CN, zh-SG, zh-HK, zh-MO, zh-TW)
        return primary == 0x04;
    }

    #[cfg(not(windows))]
    {
        for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                let lower = val.to_lowercase();
                if lower.starts_with("zh") || lower.contains("zh_cn") || lower.contains("zh_tw") {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
unsafe fn windows_lang_id() -> u16 {
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    GetUserDefaultUILanguage()
}

// ── Translation helpers ──────────────────────────────────────────

/// Returns the Chinese translation if system is Chinese, otherwise English.
fn t(en: &'static str, zh: &'static str) -> &'static str {
    if is_chinese() { zh } else { en }
}

// ── Status code translations ─────────────────────────────────────

pub fn status_format_error() -> &'static str {
    t("Status: Format Error", "状态：格式错误")
}

pub fn status_server_failure() -> &'static str {
    t("Status: Server Failure", "状态：服务器故障")
}

pub fn status_nxdomain() -> &'static str {
    t("Status: NXDomain", "状态：域名不存在")
}

pub fn status_not_implemented() -> &'static str {
    t("Status: Not Implemented", "状态：未实现")
}

pub fn status_query_refused() -> &'static str {
    t("Status: Query Refused", "状态：查询被拒绝")
}

pub fn status_bad_version() -> &'static str {
    t("Status: Bad Version", "状态：版本错误")
}

pub fn status_private_reason(num: u16) -> String {
    if is_chinese() {
        format!("状态：私有原因 ({})", num)
    } else {
        format!("Status: Private Reason ({})", num)
    }
}

pub fn status_other_failure(num: u16) -> String {
    if is_chinese() {
        format!("状态：其他故障 ({})", num)
    } else {
        format!("Status: Other Failure ({})", num)
    }
}

// ── Transport error phase labels ─────────────────────────────────

pub fn phase_protocol() -> &'static str {
    t("protocol", "协议")
}

pub fn phase_network() -> &'static str {
    t("network", "网络")
}

pub fn phase_tls() -> &'static str {
    "tls"
}

pub fn phase_http() -> &'static str {
    "http"
}

// ── Transport error messages ─────────────────────────────────────

pub fn truncated_response() -> &'static str {
    t("Truncated response", "响应被截断")
}

pub fn malformed_packet_insufficient() -> &'static str {
    t("Malformed packet: insufficient data", "数据包格式错误：数据不足")
}

pub fn malformed_packet_record_length(stated: u16, expected: u16) -> String {
    if is_chinese() {
        format!("数据包格式错误：记录长度应为{}，实际为{}", expected, stated)
    } else {
        format!("Malformed packet: record length should be {}, got {}", expected, stated)
    }
}

pub fn malformed_packet_record_length_at_least(stated: u16, expected: u16) -> String {
    if is_chinese() {
        format!("数据包格式错误：记录长度应至少为{}，实际为{}", expected, stated)
    } else {
        format!("Malformed packet: record length should be at least {}, got {}", expected, stated)
    }
}

pub fn malformed_packet_label_length(stated: u16, actual: u16) -> String {
    if is_chinese() {
        format!("数据包格式错误：声明长度{}，但读取了{}字节", stated, actual)
    } else {
        format!("Malformed packet: length {} was specified, but read {} bytes", stated, actual)
    }
}

pub fn malformed_packet_too_much_recursion(indices: &str) -> String {
    if is_chinese() {
        format!("数据包格式错误：递归过深：{:?}", indices)
    } else {
        format!("Malformed packet: too much recursion: {:?}", indices)
    }
}

pub fn malformed_packet_out_of_bounds(index: u16) -> String {
    if is_chinese() {
        format!("数据包格式错误：越界 ({})", index)
    } else {
        format!("Malformed packet: out of bounds ({})", index)
    }
}

pub fn malformed_packet_wrong_version(stated: u8, max: u8) -> String {
    if is_chinese() {
        format!("数据包格式错误：记录指定版本{}，期望最多{}", stated, max)
    } else {
        format!("Malformed packet: record specifies version {}, expected up to {}", stated, max)
    }
}

// ── Resolver errors ──────────────────────────────────────────────

pub fn unable_obtain_resolver(e: &dyn std::fmt::Display) -> String {
    if is_chinese() {
        format!("无法获取解析器：{}", e)
    } else {
        format!("Unable to obtain resolver: {}", e)
    }
}

pub fn no_nameserver_found() -> &'static str {
    t("No nameserver found", "未找到DNS服务器")
}

pub fn error_reading_network_config(e: &dyn std::fmt::Display) -> String {
    if is_chinese() {
        format!("读取网络配置出错：{}", e)
    } else {
        format!("Error reading network configuration: {}", e)
    }
}

#[allow(dead_code)]
pub fn unsupported_platform() -> &'static str {
    t(
        "dog cannot automatically detect nameservers on this platform; you will have to provide one explicitly",
        "dog无法在此平台上自动检测DNS服务器；请手动指定"
    )
}

// ── Hosts file warning ───────────────────────────────────────────

pub fn domain_in_hosts(domain: &dyn std::fmt::Display) -> String {
    if is_chinese() {
        format!("警告：域名'{}'也存在于hosts文件中", domain)
    } else {
        format!("warning: domain '{}' also exists in hosts file", domain)
    }
}

// ── Feature disabled messages ────────────────────────────────────

#[allow(dead_code)]
pub fn tls_disabled() -> &'static str {
    t(
        "dog: Cannot use '--tls': This version of dog has been compiled without TLS support",
        "dog：无法使用'--tls'：此版本的dog编译时未启用TLS支持"
    )
}

#[allow(dead_code)]
pub fn https_disabled() -> &'static str {
    t(
        "dog: Cannot use '--https': This version of dog has been compiled without HTTPS support",
        "dog：无法使用'--https'：此版本的dog编译时未启用HTTPS支持"
    )
}

// ── Short mode ───────────────────────────────────────────────────

pub fn no_results() -> &'static str {
    t("No results", "无结果")
}

// ── Misc ─────────────────────────────────────────────────────────

pub fn invalid_options_prefix() -> &'static str {
    t("dog: Invalid options: ", "dog：无效选项：")
}

pub fn nameserver_http_status(status: u16, reason: &Option<String>) -> String {
    if is_chinese() {
        format!(
            "DNS服务器返回HTTP {} ({})",
            status,
            reason.as_deref().unwrap_or("无原因")
        )
    } else {
        format!(
            "Nameserver returned HTTP {} ({})",
            status,
            reason.as_deref().unwrap_or("No reason")
        )
    }
}

// ── Usage / help text ────────────────────────────────────────────

static USAGE_EN: &str = r##"\4mUsage:\0m
  \1mdog\0m \1;33m[OPTIONS]\0m [--] \32m<arguments>\0m

\4mExamples:\0m
  \1mdog\0m \32mexample.net\0m                          Query a domain using default settings
  \1mdog\0m \32mexample.net MX\0m                       ...looking up MX records instead
  \1mdog\0m \32mexample.net MX @1.1.1.1\0m              ...using a specific nameserver instead
  \1mdog\0m \32mexample.net MX @1.1.1.1\0m \1;33m-T\0m           ...using TCP rather than UDP
  \1mdog\0m \1;33m-q\0m \33mexample.net\0m \1;33m-t\0m \33mMX\0m \1;33m-n\0m \33m1.1.1.1\0m \1;33m-T\0m   As above, but using explicit arguments

\4mQuery options:\0m
  \32m<arguments>\0m              Human-readable host names, nameservers, types, or classes
  \1;33m-q\0m, \1;33m--query\0m=\33mHOST\0m         Host name or domain name to query
  \1;33m-t\0m, \1;33m--type\0m=\33mTYPE\0m          Type of the DNS record being queried (A, MX, NS...)
  \1;33m-n\0m, \1;33m--nameserver\0m=\33mADDR\0m    Address of the nameserver to send packets to
  \1;33m--class\0m=\33mCLASS\0m            Network class of the DNS record being queried (IN, CH, HS)

\4mSending options:\0m
  \1;33m--edns\0m=\33mSETTING\0m           Whether to OPT in to EDNS (disable, hide, show)
  \1;33m--txid\0m=\33mNUMBER\0m            Set the transaction ID to a specific value
  \1;33m-Z\0m=\33mTWEAKS\0m                Set uncommon protocol-level tweaks

\4mProtocol options:\0m
  \1;33m-U\0m, \1;33m--udp\0m                Use the DNS protocol over UDP
  \1;33m-T\0m, \1;33m--tcp\0m                Use the DNS protocol over TCP
  \1;33m-S\0m, \1;33m--tls\0m                Use the DNS-over-TLS protocol
  \1;33m-H\0m, \1;33m--https\0m              Use the DNS-over-HTTPS protocol

\4mOutput options:\0m
  \1;33m-1\0m, \1;33m--short\0m              Short mode: display nothing but the first result
  \1;33m-J\0m, \1;33m--json\0m               Display the output as JSON
  \1;33m--color\0m, \1;33m--colour\0m=\33mWHEN\0m   When to colourise the output (always, automatic, never)
  \1;33m--seconds\0m                Do not format durations, display them as seconds
  \1;33m--time\0m                   Print how long the response took to arrive

\4mMeta options:\0m
  \1;33m-?\0m, \1;33m--help\0m               Print list of command-line options
  \1;33m-v\0m, \1;33m--version\0m            Print version information
  \1;33m--verbose\0m                 Print verbose diagnostic output
"##;

/// The Chinese usage template.
static USAGE_ZH: &str = concat!(
    "\\4m用法:\\0m\n",
    "  \\1mdog\\0m \\1;33m[选项]\\0m [--] \\32m<参数>\\0m\n",
    "\n",
    "\\4m示例:\\0m\n",
    "  \\1mdog\\0m \\32mexample.net\\0m                          使用默认设置查询域名\n",
    "  \\1mdog\\0m \\32mexample.net MX\\0m                       查询MX记录\n",
    "  \\1mdog\\0m \\32mexample.net MX @1.1.1.1\\0m              使用指定DNS服务器\n",
    "  \\1mdog\\0m \\32mexample.net MX @1.1.1.1\\0m \\1;33m-T\\0m           使用TCP DNS\n",
    "  \\1mdog\\0m \\1;33m-q\\0m \\33mexample.net\\0m \\1;33m-t\\0m \\33mMX\\0m \\1;33m-n\\0m \\33m1.1.1.1\\0m \\1;33m-T\\0m   与上例相同但使用显式参数\n",
    "\n",
    "\\4m查询选项:\\0m\n",
    "  \\32m<参数>\\0m                   主机名、DNS服务器、记录类型或类别\n",
    "  \\1;33m-q\\0m, \\1;33m--query\\0m=\\33mHOST\\0m         要查询的主机名或域名\n",
    "  \\1;33m-t\\0m, \\1;33m--type\\0m=\\33mTYPE\\0m          要查询的DNS记录类型 (A, MX, NS...)\n",
    "  \\1;33m-n\\0m, \\1;33m--nameserver\\0m=\\33mADDR\\0m    要请求的DNS服务器地址\n",
    "  \\1;33m--class\\0m=\\33mCLASS\\0m            要查询的网络类别 (IN, CH, HS)\n",
    "\n",
    "\\4m传输选项:\\0m\n",
    "  \\1;33m--edns\\0m=\\33mSETTING\\0m           是否启用EDNS (disable, hide, show)\n",
    "  \\1;33m--txid\\0m=\\33mNUMBER\\0m            设置事务ID为指定值\n",
    "  \\1;33m-Z\\0m=\\33mTWEAKS\\0m                设置协议级调整选项\n",
    "\n",
    "\\4m协议选项:\\0m\n",
    "  \\1;33m-U\\0m, \\1;33m--udp\\0m                使用UDP DNS\n",
    "  \\1;33m-T\\0m, \\1;33m--tcp\\0m                使用TCP DNS\n",
    "  \\1;33m-S\\0m, \\1;33m--tls\\0m                使用DNS-over-TLS\n",
    "  \\1;33m-H\\0m, \\1;33m--https\\0m              使用DNS-over-HTTPS\n",
    "\n",
    "\\4m输出选项:\\0m\n",
    "  \\1;33m-1\\0m, \\1;33m--short\\0m              简洁模式：仅输出第一个结果\n",
    "  \\1;33m-J\\0m, \\1;33m--json\\0m               以JSON输出结果\n",
    "  \\1;33m--color\\0m, \\1;33m--colour\\0m=\\33mWHEN\\0m   彩色输出配置 (always, automatic, never)\n",
    "  \\1;33m--seconds\\0m                以秒显示缓存时间\n",
    "  \\1;33m--time\\0m                   显示响应耗时\n",
    "\n",
    "\\4m全局选项:\\0m\n",
    "  \\1;33m-?\\0m, \\1;33m--help\\0m               打印命令行选项列表\n",
    "  \\1;33m-v\\0m, \\1;33m--version\\0m            打印版本信息\n",
    "  \\1;33m--verbose\\0m                打印详细诊断输出\n",
);

/// Returns the usage/help text localized and with ANSI codes processed.
pub fn get_usage_text(use_colours: bool) -> String {
    let raw = if is_chinese() { USAGE_ZH } else { USAGE_EN };
    let tagline = if is_chinese() {
        "\\1mdog\\0m \\1;32m●\\0m 命令行DNS客户端"
    } else {
        "dog \\1;32m●\\0m command-line DNS client"
    };

    if use_colours {
        format!("{}\n\n{}", convert_codes(tagline), convert_codes(raw))
    } else {
        format!("{}\n\n{}", strip_codes(tagline), strip_codes(raw))
    }
}

/// Converts the escape codes in usage templates to ANSI escape codes.
fn convert_codes(input: &str) -> String {
    input.replace('\\', "\x1B[")
}

/// Removes escape codes from usage templates.
fn strip_codes(input: &str) -> String {
    input.replace("\\0m", "")
         .replace("\\1m", "")
         .replace("\\4m", "")
         .replace("\\32m", "")
         .replace("\\33m", "")
         .replace("\\1;31m", "")
         .replace("\\1;32m", "")
         .replace("\\1;33m", "")
         .replace("\\1;4;34", "")
}
