use woothee::parser::Parser;

/// 规则表识别不出时 woothee 返回的占位值。
const UNKNOWN: &str = "UNKNOWN";

/// 从 `User-Agent` 解析 `(浏览器, 操作系统)` 名称，缺失或识别不出时对应项为 `None`。
pub fn describe(user_agent: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(user_agent) = user_agent.filter(|value| !value.trim().is_empty()) else {
        return (None, None);
    };

    let Some(result) = Parser::new().parse(user_agent) else {
        return (None, None);
    };

    (known(result.name), known(result.os))
}

fn known(value: &str) -> Option<String> {
    (value != UNKNOWN && !value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::describe;

    #[test]
    fn browser_and_os_are_extracted() {
        let chrome = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                      Chrome/141.0.0.0 Safari/537.36";
        assert_eq!(
            describe(Some(chrome)),
            (Some("Chrome".to_string()), Some("Windows 10".to_string()))
        );

        let safari = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) \
                      AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
        assert_eq!(
            describe(Some(safari)),
            (Some("Safari".to_string()), Some("iPhone".to_string()))
        );

        let firefox = "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0";
        assert_eq!(
            describe(Some(firefox)),
            (Some("Firefox".to_string()), Some("Linux".to_string()))
        );
    }

    #[test]
    fn missing_or_unrecognized_agent_yields_nothing() {
        assert_eq!(describe(None), (None, None));
        assert_eq!(describe(Some("")), (None, None));
        assert_eq!(describe(Some("   ")), (None, None));
        assert_eq!(describe(Some("custom-agent/1.0")), (None, None));
        // 规则表命中但全部字段都是占位值：与识别不出同样按空处理。
        assert_eq!(describe(Some("-")), (None, None));
    }
}
