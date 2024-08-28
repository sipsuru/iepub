use crate::{common::{IError, IResult}, mobi::core::MobiNav};

pub(crate) struct Cover(pub Vec<u8>);

impl Cover {
    pub(crate) fn write(&mut self,file:&str) ->std::io::Result<()>{
        let  image = &mut self.0;

        std::fs::write(format!("{}.{}",file,get_suffix(image)), image)?;
        Ok(())
    }
    pub(crate) fn get_file_name(&self)->String{
        let  image = &self.0;
        return format!("cover.{}",get_suffix(image));
    }
}

pub(crate) fn get_suffix(image:&[u8])->String{
    let mut suffix = "jpe";

    if b"JFIF" == &image[6..10] {
        suffix = "jpeg";
    }else if b"PNG" == &image[1..4] {
        suffix = "png";
    } else if b"GIF" == &image[0..3]{
        suffix = "gif";
    } else if b"WEBP" == &image[8..12] {
        suffix = "webp";
    }
    suffix.to_string()
}


/// 从html中获取所有的img recindex以及文件名
pub(crate) fn read_image_recindex_from_html(html:&[u8])->IResult<Vec<usize>>{
    use quick_xml::events::{BytesStart, Event};
    use quick_xml::reader::Reader;


    let mut reader = Reader::from_reader(std::io::Cursor::new(html));
    reader.config_mut().trim_text(true);
    reader.config_mut().expand_empty_elements=true;


    let mut res = Vec::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {

            Ok(Event::Start(e))=>{
                let name = String::from_utf8(e.name().as_ref().to_vec()).map_err(|_| {
                    IError::InvalidArchive("not a img")
                })?;

                if name == "img" {
                    let recindex = e.get_recindex();
                    if let Some(index) = recindex {
                        res.push(index);
                    }
                }
            }
            Ok(Event::Eof) => {
                break;
            }
            Err(e) => {
                return Err(IError::Xml(e));
            }
            _ => (),
        }
    }

    Ok(res)

}
trait AttrExt {
    fn get_recindex(&self) -> Option<usize>;
}

impl<'a> AttrExt for quick_xml::events::BytesStart<'a> {
    fn get_recindex(&self) -> Option<usize> {
        let attr = self.attributes_raw();
        let mut index = 0;
        let key = b"recindex=";

        while index < attr.len() {
            let mut now = attr[index];

            let mut j = 0;
            while j < key.len() {
                if now == key[j] {
                    index += 1;
                    now = attr[index];
                } else {
                    break;
                }
                j += 1;
            }

            if j == key.len() {
                // key 已经对上了，现在 取值
                let mut except: u8 = 0x20; // 默认期待空格
                if now == 0x27 {
                    // 单引号
                    except = 0x27;
                    index += 1;
                } else if now == 0x22 {
                    // 双引号
                    except = 0x22;
                    index += 1;
                } else if now >= 0x30 && now <= 0x39 {
                    // 数字0-9
                } else {
                    // 其他不允许，这里就直接结束掉
                    return None;
                }

                let mut res: usize = 0;
                while index < attr.len() {
                    now = attr[index];
                    if now >= 0x30 && now <= 0x39 {
                        // 数字0-9
                        res = res * 10;
                        res += (now - 0x30) as usize;
                    } else if now == 0x20 || now == 0x27 || now == 0x22 {
                        break;
                    } else {
                        return None;
                    }
                    index += 1;
                }
                if except == 0x20 {
                    // 期待空格
                    if index == attr.len() {
                        // 已经读取完了，允许该情况
                        return Some(res);
                    }
                }

                if index == attr.len() || attr[index] != except {
                    // 该有的字符没有出现
                    return None;
                }
                return Some(res);
            }
            index += 1;
        }
        None
    }
}