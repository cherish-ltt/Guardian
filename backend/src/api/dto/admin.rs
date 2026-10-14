//! 后台管理接口的查询参数与请求体。
//!
//! 请求体直接复用领域层的写入命令（它们的字段名与前端契约一致）；
//! 查询参数用 [`QueryString`] 统一解析，避免为每个列表接口重复声明页码字段。

use std::collections::HashMap;

use serde::Deserialize;

use crate::domain::{error::AppError, models::pagination::PageQuery};

/// 已解析的查询串：只做取值与类型转换，不做业务校验。
#[derive(Debug, Default)]
pub struct QueryString(HashMap<String, String>);

impl QueryString {
    pub fn parse(raw: Option<&str>) -> Self {
        let mut values = HashMap::new();
        for (key, value) in form_urlencoded_pairs(raw.unwrap_or_default()) {
            values.insert(key, value);
        }

        Self(values)
    }

    /// 分页参数：`pageNum` / `pageSize` / `sortField` / `sortOrder`。
    pub fn page(&self) -> PageQuery {
        PageQuery {
            page_num: self
                .int("pageNum")
                .ok()
                .flatten()
                .and_then(|value| u32::try_from(value).ok()),
            page_size: self
                .int("pageSize")
                .ok()
                .flatten()
                .and_then(|value| u32::try_from(value).ok()),
            sort_field: self.text("sortField"),
            sort_order: self.text("sortOrder"),
        }
    }

    /// 文本条件：空串按「未传」处理，与 Spring 的 `#{keyword} IS NULL OR #{keyword} = ''` 一致。
    pub fn text(&self, key: &str) -> Option<String> {
        self.0
            .get(key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    /// 整数条件：无法解析时返回 400，避免静默忽略过滤条件。
    pub fn int(&self, key: &str) -> Result<Option<i64>, AppError> {
        let Some(value) = self.text(key) else {
            return Ok(None);
        };

        value
            .parse::<i64>()
            .map(Some)
            .map_err(|_| AppError::business(format!("参数 {key} 必须是整数")))
    }
}

/// 代码生成批量导入请求：`{ "tableNames": ["sys_user"] }`。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorImportRequest {
    pub table_names: Option<Vec<String>>,
}

fn form_urlencoded_pairs(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key), decode(value))
        })
        .collect()
}

/// `application/x-www-form-urlencoded` 解码：`+` 表示空格，`%XX` 表示字节。
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            },
            b'%' if index + 2 < bytes.len() => {
                match u8::from_str_radix(&value[index + 1..index + 3], 16) {
                    Ok(byte) => {
                        output.push(byte);
                        index += 3;
                    },
                    Err(_) => {
                        output.push(bytes[index]);
                        index += 1;
                    },
                }
            },
            byte => {
                output.push(byte);
                index += 1;
            },
        }
    }

    String::from_utf8_lossy(&output).to_string()
}

#[cfg(test)]
mod tests {
    use super::{GeneratorImportRequest, QueryString};
    use crate::domain::error::AppError;

    #[test]
    fn page_fields_are_parsed_and_normalized_later() {
        let query = QueryString::parse(Some("pageNum=2&pageSize=20&sortField=id&sortOrder=desc"));
        let page = query.page();

        assert_eq!(page.page_num, Some(2));
        assert_eq!(page.page_size, Some(20));
        assert_eq!(page.sort_field.as_deref(), Some("id"));
        assert_eq!(page.sort_order.as_deref(), Some("desc"));
    }

    #[test]
    fn missing_or_empty_values_are_none() {
        let query = QueryString::parse(Some("keyword=&status="));

        assert_eq!(query.text("keyword"), None);
        assert_eq!(query.int("status").expect("空串不算错误"), None);
        assert_eq!(query.text("unknown"), None);
        assert_eq!(query.page().page_num, None);
    }

    #[test]
    fn values_are_url_decoded() {
        let query = QueryString::parse(Some("keyword=%E7%94%A8%E6%88%B7+%E7%AE%A1%E7%90%86"));

        assert_eq!(query.text("keyword").as_deref(), Some("用户 管理"));
    }

    #[test]
    fn invalid_integer_is_rejected_instead_of_ignored() {
        let query = QueryString::parse(Some("status=abc"));
        let error = query.int("status").expect_err("非法整数必须报错");

        assert!(matches!(error, AppError::Business(message) if message.contains("status")));
    }

    #[test]
    fn import_request_reads_table_names() {
        let request: GeneratorImportRequest =
            serde_json::from_str(r#"{"tableNames":["sys_user","sys_role"]}"#).expect("解析请求");

        assert_eq!(
            request.table_names.unwrap_or_default(),
            vec!["sys_user".to_string(), "sys_role".to_string()]
        );
    }
}
