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
    if value.contains(['"', ',', '\n', '\r']) {
        return format!("\"{}\"", value.replace('"', "\"\""));
    }

    value.to_string()
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
