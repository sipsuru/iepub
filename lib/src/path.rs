//! 解析路径
//! 不需要访问文件系统，支持相对和绝对路径
//!
#[derive(Clone)]
pub(crate) struct Path {
    /// 逐级路径
    paths: Vec<String>,
    /// home目录
    home: String,
    /// 分隔符
    sep: String,
    is_absolute:bool,
}

impl Path {
    /// 基于操作系统解析路径
    pub fn system(path: &str) -> Self {
        #[cfg(target_os = "windows")]
        let sep = "\\";
        #[cfg(not(target_os = "windows"))]
        let sep = "/";
        let mut paths = Vec::new();
        let v = path.split(sep);
        for ele in v {
            paths.push(ele.to_string());
        }

        Self {
            paths,
            sep: sep.to_string(),
            home: String::new(),
            is_absolute:path.starts_with("/")
        }
    }

    pub fn join(&self, path: &str) ->Self {

        let mut s = self.clone();

        let v = path.split(s.sep.as_str());
        for ele in v {
            if ele == ".." {
                s.paths.pop();
            } else if ele == "." {
            } else {
                s.paths.push(ele.to_string());
            }
        }
        s
    }
    pub fn to_str(&self) -> String {
        // if self.is_absolute {
        //     format!("/{}",self.paths.join(&self.sep))
        // }else{
            self.paths.join(&self.sep)
        // }
       
    }
    pub fn pop(&self)->Self{
        let mut s= self.clone();
        s.paths.pop();
        s
    }
}

#[cfg(test)]
mod tests {
    use super::Path;

    #[test]
    fn test() {
        let mut path = Path::system("/ok");
        path = path.join("1");

        assert_eq!("/ok/1", path.to_str());


        let mut path = Path::system("/ok");
        path = path.join("../1");

        assert_eq!("/1", path.to_str());

        let mut path = Path::system("res");
        path = path.join("../1");

        assert_eq!("1", path.to_str());

    }
}
