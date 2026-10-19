/// 分页请求。页码从 1 开始，每页最多 100 条。
///
/// 排序由各列表的 SQL 固定给出（如用户列表按 `id`）：接口不暴露任意列排序，
/// 避免把用户输入的列名拼进 SQL。
#[derive(Debug, Clone, Default)]
pub struct PageQuery {
    pub page_num: Option<u32>,
    pub page_size: Option<u32>,
}

pub const DEFAULT_PAGE_NUM: u32 = 1;
pub const DEFAULT_PAGE_SIZE: u32 = 10;
pub const MAX_PAGE_SIZE: u32 = 100;

/// 归一化后的分页参数，仓储层只接受它。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    page_num: u32,
    page_size: u32,
}

impl Page {
    pub fn from_query(query: Option<&PageQuery>) -> Self {
        let page_num = query
            .and_then(|query| query.page_num)
            .filter(|page_num| *page_num >= 1)
            .unwrap_or(DEFAULT_PAGE_NUM);
        let page_size = query
            .and_then(|query| query.page_size)
            .filter(|page_size| *page_size >= 1)
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .min(MAX_PAGE_SIZE);

        Self {
            page_num,
            page_size,
        }
    }

    /// 导出用：不分页，只取前 `limit` 行。
    pub fn export(limit: u32) -> Self {
        Self {
            page_num: DEFAULT_PAGE_NUM,
            page_size: limit,
        }
    }

    pub fn page_num(&self) -> u32 {
        self.page_num
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    pub fn offset(&self) -> i64 {
        i64::from(self.page_num.saturating_sub(1)) * i64::from(self.page_size)
    }

    pub fn limit(&self) -> i64 {
        i64::from(self.page_size)
    }
}

/// 分页响应，字段名与 Spring 版 `PageResult` 一致。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub rows: Vec<T>,
    pub total: i64,
    pub page_num: u32,
    pub page_size: u32,
}

impl<T> PageResult<T> {
    pub fn new(rows: Vec<T>, total: i64, page: Page) -> Self {
        Self {
            rows,
            total,
            page_num: page.page_num,
            page_size: page.page_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_PAGE_SIZE, Page, PageQuery, PageResult};

    #[test]
    fn missing_and_invalid_values_fall_back_to_defaults() {
        let page = Page::from_query(None);
        assert_eq!(page.page_num(), 1);
        assert_eq!(page.page_size(), 10);
        assert_eq!(page.offset(), 0);

        let page = Page::from_query(Some(&PageQuery {
            page_num: Some(0),
            page_size: Some(0),
        }));
        assert_eq!(page.page_num(), 1);
        assert_eq!(page.page_size(), 10);
    }

    #[test]
    fn page_size_is_capped_and_offset_follows_page() {
        let page = Page::from_query(Some(&PageQuery {
            page_num: Some(3),
            page_size: Some(1000),
        }));
        assert_eq!(page.page_size(), MAX_PAGE_SIZE);
        assert_eq!(page.page_num(), 3);
        assert_eq!(page.offset(), 200);
    }

    #[test]
    fn page_result_serializes_with_spring_field_names() {
        let result = PageResult::new(vec![1, 2], 5, Page::from_query(None));
        let json = serde_json::to_value(&result).expect("序列化分页结果");

        assert_eq!(json["pageNum"], 1);
        assert_eq!(json["pageSize"], 10);
        assert_eq!(json["total"], 5);
        assert_eq!(json["rows"].as_array().map(Vec::len), Some(2));
    }
}
