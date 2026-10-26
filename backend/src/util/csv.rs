/// 按 Spring 版 `SystemManagementService.csv()` 的规则拼 CSV：
/// 单元格含逗号、引号或换行时整体加引号，内部引号翻倍；行尾统一 `\n`。
pub fn to_csv(headers: &[&str], rows: &[Vec<Option<String>>]) -> String {
    let mut output = String::new();
    output.push_str(&headers.join(","));
    output.push('\n');

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&escape_cell(cell.as_deref().unwrap_or_default()));
        }
        output.push('\n');
    }

    output
}

fn escape_cell(value: &str) -> String {
    let value = guard_formula(value);
    let value = value.as_ref();
    if value.contains(['"', ',', '\n', '\r']) {
        return format!("\"{}\"", value.replace('"', "\"\""));
    }

    value.to_string()
}

/// 以 `= + - @` 或制表符 / 回车开头的文本会被表格软件当成公式执行。
///
/// 用户昵称、请求参数这类字段完全由用户控制，导出后双击打开就可能触发
/// 外链与命令。这类单元格前面加一个单引号，让它保持文本。
fn guard_formula(value: &str) -> std::borrow::Cow<'_, str> {
    if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        return std::borrow::Cow::Owned(format!("'{value}"));
    }

    std::borrow::Cow::Borrowed(value)
}

/// 数值单元格：空值转空字符串，其余按十进制输出。
pub fn number_cell(value: Option<i64>) -> Option<String> {
    value.map(|value| value.to_string())
}

/// 0 显示为「停用 / 失败」，其余显示为「正常 / 成功」。
pub fn status_text(value: Option<i64>) -> Option<String> {
    value.map(|value| {
        if value == 1 {
            "正常".to_string()
        } else {
            "停用".to_string()
        }
    })
}

/// 日志结果文案：1 为成功，其余为失败。
pub fn result_text(value: Option<i64>) -> Option<String> {
    value.map(|value| {
        if value == 1 {
            "成功".to_string()
        } else {
            "失败".to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{number_cell, result_text, status_text, to_csv};

    #[test]
    fn plain_row_is_joined_with_commas() {
        let csv = to_csv(
            &["登录账号", "用户昵称"],
            &[vec![Some("admin".to_string()), Some("管理员".to_string())]],
        );

        assert_eq!(csv, "登录账号,用户昵称\nadmin,管理员\n");
    }

    #[test]
    fn cells_with_separators_are_quoted_and_escaped() {
        let csv = to_csv(
            &["a", "b"],
            &[vec![
                Some("x,y".to_string()),
                Some("he said \"hi\"".to_string()),
            ]],
        );

        assert_eq!(csv, "a,b\n\"x,y\",\"he said \"\"hi\"\"\"\n");
    }

    /// 以公式前缀开头的文本不得被表格软件当成公式执行。
    #[test]
    fn formula_prefixes_are_neutralized() {
        let csv = to_csv(
            &["a", "b", "c", "d", "e"],
            &[vec![
                Some("=1+1".to_string()),
                Some("+SUM(A1)".to_string()),
                Some("-2+3".to_string()),
                Some("@cmd".to_string()),
                Some("\t=1".to_string()),
            ]],
        );

        assert_eq!(csv, "a,b,c,d,e\n'=1+1,'+SUM(A1),'-2+3,'@cmd,'	=1\n");
    }

    /// 普通文本与负数文案不受影响。
    #[test]
    fn ordinary_text_keeps_its_prefix() {
        let csv = to_csv(&["a"], &[vec![Some("管理员".to_string())]]);
        assert_eq!(csv, "a\n管理员\n");
    }

    #[test]
    fn missing_values_become_empty_cells() {
        let csv = to_csv(&["a", "b"], &[vec![None, Some("1".to_string())]]);
        assert_eq!(csv, "a,b\n,1\n");
    }

    #[test]
    fn status_and_result_text_follow_spring_wording() {
        assert_eq!(status_text(Some(1)).as_deref(), Some("正常"));
        assert_eq!(status_text(Some(0)).as_deref(), Some("停用"));
        assert_eq!(result_text(Some(1)).as_deref(), Some("成功"));
        assert_eq!(result_text(Some(0)).as_deref(), Some("失败"));
        assert_eq!(status_text(None), None);
    }

    /// 数值列保留原值（含 0 与负数），空值不写成 "0"。
    #[test]
    fn number_cell_renders_decimal_or_none() {
        assert_eq!(number_cell(Some(0)).as_deref(), Some("0"));
        assert_eq!(number_cell(Some(-12)).as_deref(), Some("-12"));
        assert_eq!(number_cell(None), None);
    }
}
