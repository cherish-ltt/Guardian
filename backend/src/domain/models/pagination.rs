use crate::domain::error::AppError;

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
    /// 归一化分页参数。
    ///
    /// 超出上限的 `pageSize` 直接拒绝而不是静默截断：截断会让客户端以为拿到了整页，
    /// 分页总数与实际行数对不上。缺省与非法值（0）仍按默认值处理。
    pub fn from_query(query: Option<&PageQuery>) -> Result<Self, AppError> {
        let page_num = query
            .and_then(|query| query.page_num)
            .filter(|page_num| *page_num >= 1)
            .unwrap_or(DEFAULT_PAGE_NUM);
        let page_size = query
            .and_then(|query| query.page_size)
            .filter(|page_size| *page_size >= 1)
            .unwrap_or(DEFAULT_PAGE_SIZE);
        if page_size > MAX_PAGE_SIZE {
            return Err(AppError::business(format!(
                "每页条数不能超过{MAX_PAGE_SIZE}"
            )));
        }

        Ok(Self {
            page_num,
            page_size,
        })
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
        let page = Page::from_query(None).expect("默认分页");
        assert_eq!(page.page_num(), 1);
        assert_eq!(page.page_size(), 10);
        assert_eq!(page.offset(), 0);

        let page = Page::from_query(Some(&PageQuery {
            page_num: Some(0),
            page_size: Some(0),
        }))
        .expect("非法值取默认");
        assert_eq!(page.page_num(), 1);
        assert_eq!(page.page_size(), 10);
    }

    #[test]
    fn oversized_page_size_is_rejected_instead_of_truncated() {
        let error = Page::from_query(Some(&PageQuery {
            page_num: Some(3),
            page_size: Some(MAX_PAGE_SIZE + 1),
        }))
        .expect_err("超过上限必须拒绝");

        assert_eq!(error.http_code(), 400);
        assert_eq!(error.to_string(), "每页条数不能超过100");
    }

    #[test]
    fn offset_follows_page() {
        let page = Page::from_query(Some(&PageQuery {
            page_num: Some(3),
            page_size: Some(MAX_PAGE_SIZE),
        }))
        .expect("上限内的分页");

        assert_eq!(page.page_size(), MAX_PAGE_SIZE);
        assert_eq!(page.page_num(), 3);
        assert_eq!(page.offset(), 200);
    }

    #[test]
    fn page_result_serializes_with_spring_field_names() {
        let result = PageResult::new(vec![1, 2], 5, Page::from_query(None).expect("默认分页"));
        let json = serde_json::to_value(&result).expect("序列化分页结果");

        assert_eq!(json["pageNum"], 1);
        assert_eq!(json["pageSize"], 10);
        assert_eq!(json["total"], 5);
        assert_eq!(json["rows"].as_array().map(Vec::len), Some(2));
    }
}
