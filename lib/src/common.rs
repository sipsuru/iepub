use std::{borrow::Cow, collections::HashMap, ops::Deref, string::FromUtf8Error};
#[macro_export]
macro_rules! cache_struct{
    (
     // meta data about struct
     $(#[$meta:meta])*
     $vis:vis struct $struct_name:ident {
        $(
        // meta data about field
        $(#[$field_meta:meta])*
        $field_vis:vis $field_name:ident : $field_type:ty
        ),*$(,)?
    }
    ) => {
            #[cfg(feature = "cache")]
            #[derive(serde::Deserialize,serde::Serialize)]
            $(#[$meta])*
            pub struct $struct_name{
                $(
                    $(#[$field_meta])*
                    $field_vis $field_name : $field_type,
                )*
            }
            #[cfg(not(feature = "cache"))]
            $(#[$meta])*
            pub struct $struct_name{
                $(
                    $(#[$field_meta])*
                    $field_vis $field_name : $field_type,
                )*

            }

    }
}
#[macro_export]
macro_rules! cache_enum {
    // 基础枚举匹配
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident { $($body:tt)* }) => {
        #[derive(Debug)]
        #[cfg(not(feature="cache"))]
        $(#[$meta])*
        $vis enum $name { $($body)* }

        #[derive(Debug,serde::Deserialize,serde::Serialize)]
        #[cfg(feature="cache")]
        $(#[$meta])*
        $vis enum $name { $($body)* }
    };

    // 带显式属性的枚举
    ($(#[$meta:meta])* enum $name:ident { $($body:tt)* }) => {
        $(#[$meta])*
        #[derive(Debug, Default)]
        enum $name { $($body)* }
    };

    // 支持泛型枚举
    ($(#[$meta:meta])* enum $name:ident<$T:ident> { $($body:tt)* }) => {
        $(#[$meta])*
        #[derive(Debug)]
        enum $name<$T> { $($body)* }
    };
}

///
/// 错误
///
#[derive(Debug)]
pub enum IError {
    /// io 错误
    Io(std::io::Error),
    /// invalid Zip archive: {0}
    InvalidArchive(Cow<'static, str>),

    /// unsupported Zip archive: {0}
    UnsupportedArchive(&'static str),

    /// specified file not found in archive
    FileNotFound,

    /// The password provided is incorrect
    InvalidPassword,

    Utf8(std::string::FromUtf8Error),

    Xml(quick_xml::Error),
    NoNav(&'static str),
    Cover(String),
    #[cfg(feature = "cache")]
    Cache(String),
    Unknown,
}

#[cfg(feature = "cache")]
impl From<serde_json::Error> for IError {
    fn from(value: serde_json::Error) -> Self {
        Self::Cache(format!("{:?}", value))
    }
}

impl std::fmt::Display for IError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for IError {}

pub type IResult<T> = Result<T, IError>;

impl From<std::io::Error> for IError {
    fn from(value: std::io::Error) -> Self {
        IError::Io(value)
    }
}
impl From<quick_xml::Error> for IError {
    fn from(value: quick_xml::Error) -> Self {
        match value {
            quick_xml::Error::Io(e) => IError::Io(std::io::Error::other(e)),
            _ => IError::Xml(value),
        }
    }
}

impl From<FromUtf8Error> for IError {
    fn from(value: FromUtf8Error) -> Self {
        IError::Utf8(value)
    }
}
cache_struct! {
    #[derive(Debug, Default)]
    pub(crate) struct BookInfo {
        /// 书名
        pub(crate) title: String,

        /// 标志，例如imbi
        pub(crate) identifier: String,
        /// 作者
        pub(crate) creator: Option<String>,
        ///
        /// 简介
        ///
        pub(crate) description: Option<String>,
        /// 文件创建者
        pub(crate) contributor: Option<String>,

        /// 出版日期
        pub(crate) date: Option<String>,

        /// 格式?
        pub(crate) format: Option<String>,
        /// 出版社
        pub(crate) publisher: Option<String>,
        /// 主题？
        pub(crate) subject: Option<String>,
    }
}
impl BookInfo {
    pub(crate) fn append_creator(&mut self, v: &str) {
        if let Some(c) = &mut self.creator {
            c.push_str(",");
            c.push_str(v);
        } else {
            self.creator = Some(String::from(v));
        }
    }
}

/// 去除html的标签，只保留纯文本
///
/// # Examples
///
/// ```ignore
/// assert_eq!("12345acd", unescape_html("<div><p>12345</p><p>acd</p></div>"));
/// ```
///
pub(crate) fn unescape_html(v: &str) -> String {
    let mut reader = quick_xml::reader::Reader::from_str(v);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut txt = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Text(e)) => {
                // let _= txt_buf(&e);
                if let Ok(t) = e.unescape() {
                    txt.push_str(&t.deref());
                }
            }
            Ok(quick_xml::events::Event::Eof) => {
                break;
            }
            _ => (),
        }
        buf.clear();
    }
    txt
}

pub struct DateTimeFormater {
    timestamp: u64,
    start_year: u64,
    format_map: HashMap<char, fn(u64) -> String>,
    /// 时区，默认为0
    timezone_offset: i16,
}

impl DateTimeFormater {
    pub fn custom_start(timestamp: u64, start_year: u64) -> Self {
        // 需要强制指定类型，否则自动推测会出错
        let t: fn(u64) -> String = Self::format_year;
        let t2: fn(u64) -> String = Self::format_day;

        Self {
            start_year,
            timezone_offset: 0,
            timestamp,
            format_map: HashMap::from([
                ('Y', t),
                ('M', t2),
                ('d', t2),
                ('H', t2),
                ('m', t2),
                ('s', t2),
            ]),
        }
    }
    ///
    ///
    /// # Params
    /// - timestamp 秒级时间戳
    ///
    pub fn new(timestamp: u64) -> Self {
        Self::custom_start(timestamp, 1970)
    }

    pub fn default() -> Self {
        Self::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|v| v.as_secs())
                .unwrap_or(0),
        )
    }

    pub fn with_timezone_offset(mut self, offset: i16) -> Self {
        self.timezone_offset = offset;
        self
    }

    ///
    /// 格式化
    ///
    /// %Y - 2024
    ///
    /// %M - 02
    ///
    /// %d - 03
    ///
    /// %H - 03
    ///
    /// %m - 01
    ///
    /// %s - 03
    ///
    ///
    pub fn format<T: AsRef<str>>(&self, pattern: T) -> String {
        let (year, month, day, hour, min, sec) =
            self.do_time_display(self.timestamp, self.start_year);
        let values = HashMap::from([
            ('Y', year),
            ('M', month),
            ('d', day),
            ('H', hour),
            ('m', min),
            ('s', sec),
        ]);

        let mut result = String::new();
        let mut chars = pattern.as_ref().chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' {
                if let Some(&next_c) = chars.peek() {
                    if let Some(formatter) = self.format_map.get(&next_c) {
                        result.push_str(&formatter(*values.get(&next_c).unwrap_or(&0)));
                        chars.next(); // 跳过已处理的占位符
                        continue;
                    }
                }
            }
            result.push(c);
        }
        result
    }

    pub fn default_format(&self) -> String {
        self.format("%Y-%M-%dT%H:%m:%sZ")
    }

    fn format_year(value: u64) -> String {
        format!("{:04}", value)
    }

    fn format_day(value: u64) -> String {
        format!("{:02}", value)
    }

    /// 秒级时间戳转换，支持从不同年份开始计算
    fn do_time_display(&self, value: u64, start_year: u64) -> (u64, u64, u64, u64, u64, u64) {
        // 先粗略定位到哪一年
        // 以 365 来计算，年通常只会相比正确值更晚，剩下的秒数也就更多，并且有可能出现需要往前一年的情况

        // 加上时区偏移
        let offset = self.timezone_offset * 60 * 60;

        let value = if offset < 0 {
            value - ((offset * -1) as u64)
        } else {
            value + (offset as u64)
        };

        let per_year_sec = 365 * 24 * 60 * 60; // 平年的秒数

        let mut year = value / per_year_sec;
        // 剩下的秒数，如果这些秒数 不够填补闰年，比如粗略计算是 2024年，还有 86300秒，不足一天，那么中间有很多闰年，所以 年应该-1，只有-1，因为-2甚至更多 需要 last_sec > 365 * 86400，然而这是不可能的
        let last_sec = value - (year) * per_year_sec;
        year += start_year;

        let mut leap_year_sec = 0;
        // 计算中间有多少闰年，当前年是否是闰年不影响回退，只会影响后续具体月份计算
        for y in start_year..year {
            if Self::is_leap(y) {
                // 出现了闰年
                leap_year_sec += 86400;
            }
        }
        if last_sec < leap_year_sec {
            // 不够填补闰年，年份应该-1
            year -= 1;
            // 上一年是闰年，所以需要补一天
            if Self::is_leap(year) {
                leap_year_sec -= 86400;
            }
        }
        // 剩下的秒数
        let mut time = value - leap_year_sec - (year - start_year) * per_year_sec;

        // 平年的月份天数累加
        let mut day_of_year: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        // 找到了 计算日期
        let sec = time % 60;
        time /= 60;
        let min = time % 60;
        time /= 60;
        let hour = time % 24;
        time /= 24;

        // 计算是哪天，因为每个月不一样多，所以需要修改
        if Self::is_leap(year) {
            day_of_year[1] += 1;
        }
        let mut month = 0;
        for (index, ele) in day_of_year.iter().enumerate() {
            if &time < ele {
                month = index + 1;
                time += 1; // 日期必须加一，否则 每年的 第 1 秒就成了第0天了
                break;
            }
            time -= ele;
        }

        (year, month as u64, time, hour, min, sec)
    }

    //
    // 判断是否是闰年
    //
    fn is_leap(year: u64) -> bool {
        return year % 4 == 0 && ((year % 100) != 0 || year % 400 == 0);
    }
}

// /// 时间戳转换，从1970年开始
// pub(crate) fn time_display(value: u64) -> String {
//     do_time_display(value, 1970)
// }

// ///
// /// 输出当前时间格式化
// ///
// /// 例如：
// /// 2023-09-28T09:32:24Z
// ///
// pub(crate) fn time_format() -> String {
//     // 获取当前时间戳
//     let time = std::time::SystemTime::now()
//         .duration_since(std::time::UNIX_EPOCH)
//         .map(|v| v.as_secs())
//         .unwrap_or(0);

//     time_display(time)
// }

pub(crate) fn get_media_type(file_name: &str) -> String {
    let f = file_name.to_lowercase();

    let mut types = std::collections::HashMap::new();
    types.insert(".gif", String::from("image/gif"));
    types.insert(".jpg", String::from("image/jpeg"));
    types.insert(".jpeg", String::from("image/jpeg"));
    types.insert(".png", String::from("image/png"));
    types.insert(".svg", String::from("image/svg+xml"));
    types.insert(".webp", String::from("image/webp"));
    types.insert(".mp3", String::from("audio/mpeg"));
    types.insert(".mp4", String::from("audio/mp4"));
    types.insert(".css", String::from("text/css"));
    types.insert(".ttf", String::from("application/font-sfnt"));
    types.insert(".oft", String::from("application/font-sfnt"));
    types.insert(".woff", String::from("application/font-woff"));
    types.insert(".woff", String::from("font/woff2"));
    types.insert(".xhtml", String::from("application/xhtml+xml"));
    types.insert(".js", String::from("application/javascript"));
    types.insert(".opf", String::from("application/x-dtbncx+xml"));
    let x: &[_] = &['.'];
    if let Some(index) = f.rfind(x) {
        let sub = &f[index..f.len()];
        return match types.get(&sub) {
            Some(t) => String::from(t),
            None => String::new(),
        };
    };

    String::new()
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::common::DateTimeFormater;

    pub fn get_req(url: &str) -> minreq::Request {
        let mut req = minreq::get(url);
        if let Ok(proxy) = std::env::var("HTTPS_PROXY")
            .or_else(|_e| std::env::var("https_proxy"))
            .or_else(|_e| std::env::var("ALL_PROXY"))
            .or_else(|_e| std::env::var("all_proxy"))
        {
            req = req.with_proxy(minreq::Proxy::new(proxy).expect("invalid proxy env"));
        }
        req
    }

    pub fn download_epub_file(name: &str, url: &str) {
        use super::IError;
        use std::borrow::Cow;
        if name.contains("/") {
            let p = std::path::Path::new(&name);

            std::fs::create_dir_all(format!("{}", p.parent().unwrap().display())).unwrap();
        }
        if std::fs::metadata(name).is_err() {
            // 下载并解压
            get_req(url)
                .send()
                .map_err(|e| IError::InvalidArchive(Cow::from("download fail")))
                .map(|v| (v.headers["content-length"].clone(), v.as_bytes().to_vec()))
                .and_then(|(len, res)| {
                    if len.parse::<usize>().unwrap() == res.len() {
                        Ok(res)
                    } else {
                        Err(IError::InvalidArchive(Cow::from("download fail,len error")))
                    }
                })
                .and_then(|f| std::fs::write(name, f).map_err(|e| IError::Io(e)))
                .unwrap();
        }
    }

    pub fn download_zip_file(name: &str, url: &str) -> String {
        use super::IError;
        use std::{borrow::Cow, io::Read};
        let out = format!("../target/{name}");
        if std::fs::metadata(&out).is_err() {
            // 下载并解压

            let mut zip = get_req(url)
                .send()
                .map_err(|e| IError::InvalidArchive(Cow::from("download fail")))
                .map(|v| (v.headers["content-length"].clone(), v.as_bytes().to_vec()))
                .and_then(|(len, res)| {
                    if len.parse::<usize>().unwrap() == res.len() {
                        Ok(res)
                    } else {
                        Err(IError::InvalidArchive(Cow::from("download fail,len error")))
                    }
                })
                .and_then(|f| {
                    zip::ZipArchive::new(std::io::Cursor::new(f))
                        .map_err(|e| IError::InvalidArchive(Cow::from("download fail")))
                })
                .unwrap();
            let mut zip = zip.by_name(name).unwrap();
            let mut v = Vec::new();
            zip.read_to_end(&mut v).unwrap();

            if name.contains("/") {
                std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap()).unwrap();
            }
            std::fs::write(std::path::Path::new(&out), &mut v).unwrap();
        }
        out
    }

    #[test]
    fn test_time_format() {
        assert_eq!(
            "2025-07-29T11:41:46Z",
            DateTimeFormater::new(1753760506)
                .with_timezone_offset(8)
                .default_format()
        );

        assert_eq!(
            "2025",
            DateTimeFormater::new(1753760506)
                .with_timezone_offset(8)
                .format("%Y")
        );

        assert_eq!(
            "2025-07-01T22:00:00Z",
            DateTimeFormater::new(1751407200).default_format()
        );
        assert_eq!(
            "2025-07-02T06:00:00Z",
            DateTimeFormater::new(1751407200)
                .with_timezone_offset(8)
                .default_format()
        );

        assert_eq!(
            "2025-07-01T14:00:00Z",
            DateTimeFormater::new(1751407200)
                .with_timezone_offset(-8)
                .default_format()
        );
    }
}
