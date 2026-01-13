use log::error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Response<T>
where
    T: Serialize,
{
    code: u32,
    msg: Option<String>,
    data: Option<T>, // 直接存储 T 类型，不是字符串
}

impl<T> Response<T>
where
    T: Serialize,
{
    pub fn ok(msg: Option<String>, data: T) -> Self {
        Self {
            code: 200,
            msg,
            data: Some(data),
        }
    }

    pub fn ok_msg(msg: Option<String>) -> Self {
        Self {
            code: 200,
            msg,
            data: None,
        }
    }

    pub fn ok_data(data: T) -> Self {
        Self {
            code: 200,
            msg: None,
            data: Some(data),
        }
    }

    pub fn quick_ok() -> Self
    where
        T: Default,
    {
        Self {
            code: 200,
            msg: None,
            data: None,
        }
    }

    pub fn failed(msg: String) -> Self
    where
        T: Default,
    {
        Self {
            code: 1000,
            msg: Some(msg),
            data: None,
        }
    }

    pub fn quick_failed() -> Self
    where
        T: Default,
    {
        Self {
            code: 1001,
            msg: None,
            data: None,
        }
    }
}

// 为无数据响应提供便捷方法
impl Response<()> {
    pub fn ok_msg_only(msg: String) -> Self {
        Self {
            code: 200,
            msg: Some(msg),
            data: None,
        }
    }

    pub fn failed_msg_only(msg: String) -> Self {
        Self {
            code: 1000,
            msg: Some(msg),
            data: None,
        }
    }
}

// 实现 Default trait 以便于使用
impl<T> Default for Response<T>
where
    T: Serialize + Default,
{
    fn default() -> Self {
        Self {
            code: 200,
            msg: None,
            data: Some(T::default()),
        }
    }
}
