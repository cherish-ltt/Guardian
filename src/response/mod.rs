use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseCode {
    Success = 200,
    GenericError = 1000,
    UnknownError = 1001,
    InternalError = 17000,
    ValidationError = 17001,
    AuthError = 17002,
    TokenExpired = 17003,
    PermissionDenied = 17004,
    ResourceNotFound = 17005,
    RateLimitExceeded = 17006,
    TwoFaVerificationFailed = 17007,
    InvalidTwoFaCode = 17008,
    TwoFaNotEnabled = 17009,
    TwoFaAlreadyEnabled = 17010,
}

impl ResponseCode {
    pub fn code(&self) -> u32 {
        *self as u32
    }

    pub fn msg(&self) -> &'static str {
        match self {
            ResponseCode::Success => "success",
            ResponseCode::GenericError => "通用请求失败",
            ResponseCode::UnknownError => "未知错误",
            ResponseCode::InternalError => "系统内部错误",
            ResponseCode::ValidationError => "参数验证失败",
            ResponseCode::AuthError => "认证失败",
            ResponseCode::TokenExpired => "令牌过期",
            ResponseCode::PermissionDenied => "权限不足",
            ResponseCode::ResourceNotFound => "资源不存在",
            ResponseCode::RateLimitExceeded => "请求频率过高",
            ResponseCode::TwoFaVerificationFailed => "2FA验证失败",
            ResponseCode::InvalidTwoFaCode => "无效的2FA验证码",
            ResponseCode::TwoFaNotEnabled => "未启用2FA",
            ResponseCode::TwoFaAlreadyEnabled => "已启用2FA",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Response<T>
where
    T: Serialize,
{
    pub code: u32,
    pub msg: Option<String>,
    pub data: Option<T>,
    pub timestamp: i64,
}

impl<T> Response<T>
where
    T: Serialize,
{
    pub fn new(code: u32, msg: Option<String>, data: Option<T>) -> Self {
        Self {
            code,
            msg,
            data,
            timestamp: Local::now().timestamp_millis(),
        }
    }

    pub fn ok(msg: Option<String>, data: T) -> Self {
        Self::new(ResponseCode::Success.code(), msg, Some(data))
    }

    pub fn ok_msg(msg: Option<String>) -> Self
    where
        T: Default,
    {
        Self::new(ResponseCode::Success.code(), msg, None)
    }

    pub fn ok_data(data: T) -> Self {
        Self::new(ResponseCode::Success.code(), None, Some(data))
    }

    pub fn quick_ok() -> Self
    where
        T: Default,
    {
        Self::new(ResponseCode::Success.code(), None, None)
    }

    pub fn failed(msg: String) -> Self
    where
        T: Default,
    {
        Self::new(ResponseCode::GenericError.code(), Some(msg), None)
    }

    pub fn quick_failed() -> Self
    where
        T: Default,
    {
        Self::new(ResponseCode::UnknownError.code(), None, None)
    }

    pub fn from_code(code: ResponseCode, msg: Option<String>) -> Self
    where
        T: Default,
    {
        let final_msg = msg.or_else(|| Some(code.msg().to_string()));
        Self::new(code.code(), final_msg, None)
    }

    pub fn from_code_with_data(code: ResponseCode, msg: Option<String>, data: T) -> Self {
        let final_msg = msg.or_else(|| Some(code.msg().to_string()));
        Self::new(code.code(), final_msg, Some(data))
    }
}

impl ResponseCode {
    pub fn to_response<T>(self, msg: Option<String>) -> Response<T>
    where
        T: Serialize + Default,
    {
        Response::from_code(self, msg)
    }

    pub fn to_response_with_data<T>(self, msg: Option<String>, data: T) -> Response<T>
    where
        T: Serialize,
    {
        Response::from_code_with_data(self, msg, data)
    }
}

impl<T> From<ResponseCode> for Response<T>
where
    T: Serialize + Default,
{
    fn from(code: ResponseCode) -> Self {
        Response::from_code(code, None)
    }
}

impl Response<()> {
    pub fn ok_msg_only(msg: String) -> Self {
        Self::new(ResponseCode::Success.code(), Some(msg), None)
    }

    pub fn failed_msg_only(msg: String) -> Self {
        Self::new(ResponseCode::GenericError.code(), Some(msg), None)
    }
}

impl<T> Default for Response<T>
where
    T: Serialize + Default,
{
    fn default() -> Self {
        Self {
            code: ResponseCode::Success.code(),
            msg: None,
            data: Some(T::default()),
            timestamp: Local::now().timestamp_millis(),
        }
    }
}
