use std::{env, io::Write};

use iepub::reader::read_from_file;

fn main() {


    let s:Vec<String>= env::args().collect();

    if s.len() <2 {
        println!("usage: {} file",s[0]);
        return;
    }

    let file = &s[1];

    match read_from_file(file.as_str()) {
        Ok(mut book) => {

            println!("title = {}",book.title());
            // println!("{}",book);
            println!("{:?}",book.cover());
            println!("des {:?}",book.description());
            println!("cre{:?}",book.creator());
            if let Some(cover) = book.cover(){
                println!("cover = {:?}",cover);
                // 提取cover
                
                if let Some(data)  = cover.data() {
                    let mut fs = std::fs::File::create("cover.jpg").unwrap();
                    fs.write_all(data).unwrap();
                }
                


            }
        },
        Err(e) => {
            println!("{:?}",e);
        },
    }

}
