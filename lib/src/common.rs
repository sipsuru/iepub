use std::str::FromStr;

#[derive(Debug)]
pub enum LinkRel {
    CSS,
    OTHER,
}

#[macro_export]
macro_rules! epub_base_field{
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

            $(#[$meta])*
            pub struct $struct_name{

                id:String,
                _file_name:String,
                media_type:String,
                _data: Option<Vec<u8>>,
                reader:Option<std::rc::Rc<std::cell::RefCell< Box<dyn EpubReaderTrait>>>>,
                $(
                    $(#[$field_meta])*
                    $field_vis $field_name : $field_type,
                )*

            }

            impl $struct_name {
                ///
                /// 文件路径
                ///
                /// 注意，如果是 EPUB 目录下的文件，返回的时候不会带有EPUB路径
                ///
                pub fn file_name(&self)->&str{
                    self._file_name.as_str()
                }
                ///
                /// 设置文件路径
                ///
                pub fn set_file_name(&mut self,value: &str){
                    self._file_name.clear();
                    self._file_name.push_str(value);
                }

                pub fn id(&self)->&str{
                    self.id.as_str()
                }
                pub fn set_id(&mut self,id:&str){
                    self.id.clear();
                    self.id.push_str(id);
                }

                pub fn set_data(&mut self, data: Vec<u8>) {
                    // if let Some(d) = &mut self._data {
                    //     d.clear();
                    //     d.append(data);
                    // }else{
                        self._data = Some(data);
                    // }
                }
                pub fn with_file_name(mut self,value:&str)->Self{
                    self.set_file_name(value);
                    self
                }

                pub fn with_data(mut self, value:Vec<u8>)->Self{
                    self.set_data(value);
                    self
                }

            }


    }
}

pub static EPUB: &str = "EPUB/";
pub static TOC: &str = "EPUB/toc.ncx";
pub static NAV: &str = "EPUB/nav.xhtml";
pub static COVER: &str = "EPUB/cover.xhtml";
pub static OPF: &str = "EPUB/content.opf";

impl std::fmt::Display for LinkRel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::CSS => {
                    "stylesheet"
                }
                Self::OTHER => {
                    "other"
                }
            }
        )
    }
}
///
/// 输出当前时间格式化
///
/// 例如：
/// 2023-09-28T09:32:24Z
///
pub(crate) fn time_format()->String{
    // 获取当前时间戳
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0);
    
    do_time_format(time)
}


fn do_time_format(value:u64)->String{
    // 获取当前时间戳
    let mut time = value;
    let per_year_sec = 365 * 24 * 60 * 60; // 平年的秒数

    // 平年的月份天数累加
    // let mut day_of_year: [u64; 13] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];
    // 平年的月份天数累加
    let mut day_of_year: [u64; 12] = [31, 28, 31, 30, 31, 30, 31,31,30,31,30,31];

    //
    // 判断是否是闰年
    //
    #[inline]
    fn is_leap(year: u64) -> bool {
        return year % 4 == 0 && ((year % 100) != 0 || year % 400 == 0);
    }

    let mut all_sec = 0;
    // 直接算到 2038年，把每一年的秒数加起来看哪年合适
    for year in 1970..2038 {
        let is_leap = is_leap(year);

        let before_sec = all_sec;
        all_sec += per_year_sec;
        if is_leap {
            all_sec += 86400;
        }
        // println!("all={all_sec} before_sec={before_sec} year={year}");
        // 具体是哪一年应该是 当 小于这一年的秒数
        if time < all_sec {
            // 减去到上一年年底的秒数 剩下的才是这一年内的秒数
            time = value - before_sec;
            // 找到了 计算日期
            let sec = time % 60;
            time /= 60;
            let min = time % 60;
            time /= 60;
            let hour = time % 24;
            time /= 24;

            // 计算是哪天，因为每个月不一样多，所以需要修改
            if is_leap {
                day_of_year[1] += 1;
            }
            let mut month = 0;
            for (index, ele) in day_of_year.iter().enumerate() {
                if &time < ele {
                    month = index + 1;
                    time += 1;// 日期必须加一，否则 每年的 第 1 秒就成了第0天了
                    break;
                }
                time -= ele;
            }

            return format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                year, month, time, hour, min, sec
            );
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use crate::common::do_time_format;


    #[test]
    fn test_time_format(){

        assert_eq!("2024-08-05T05:39:05Z",do_time_format(1722836345));
        assert_eq!("1970-01-01T00:00:01Z",do_time_format(1));
        assert_eq!("2027-08-13T00:00:00Z",do_time_format(1818115200));
        assert_eq!("2027-08-17T23:59:59Z",do_time_format(1818547199));
        assert_eq!("1984-06-24T08:29:42Z",do_time_format(456913782));

        
        
    }
}
