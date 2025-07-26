use std::fs;

fn replace(data: String) -> String {
    data.replace("nongnong", "浓浓")
        .replace("guntang", "滚烫")
        .replace("rou体", "肉体")
        .replace("yin糜", "淫靡")
        .replace("yin心", "淫心")
        .replace("yuhuo", "欲火")
        .replace("yin声", "淫声")
        .replace("yin欲", "淫欲")
        .replace("rou欲", "肉欲")
        .replace("嫩rou", "嫩肉")
        .replace("rufang", "乳房")
        .replace("rou缝", "肉缝")
        .replace("cao", "操")
        .replace("roubang", "肉棒")
        .replace("jiba", "肉棒")
        .replace("xue", "穴")
        .replace("sao", "骚")
        .replace("guitou", "龟头")
        .replace("rou", "肉")
        .replace("taonong", "套弄")
        .replace("yin荡", "淫荡")
        .replace("rutou", "乳头")
        .replace("jian", "奸")
        .replace("mama", "妈妈")
        .replace("meimei", "妹妹")
        .replace("yin妇", "淫妇")
        .replace("shuangsi", "爽死")
        .replace("yindao", "阴道")
        .replace("yin水", "淫水")
        .replace("zuoai", "做爱")
        .replace("xiao穴", "小穴")
        .replace("luanlun", "乱伦")
        .replace("rujiao", "乳交")
        .replace("jingye", "精液")
        .replace("koujiao", "口交")
        .replace("yinnang", "阴囊")
        .replace("shuangma", "爽吗")
        .replace("makou", "马口")
        .replace("yin虐", "淫虐")
        .replace("jiejie", "姐姐")
        .replace("zigong", "子宫")
        .replace("yinjing", "阴茎")
        .replace("yinhe", "阴核")
        .replace("yingying", "硬硬")
        .replace("xiele", "泄了")
        .replace("jingzi", "精子")
        .replace("yin媚", "淫媚")
        .replace("yin乱", "淫乱")
        .replace("yin浪", "淫浪")
        .replace("yin弄", "淫弄")
        .replace("yin念", "淫念")
        .replace("yin娃", "淫娃")
        .replace("yin液", "淫液")
        .replace("", "")
        .replace("", "")
        .replace("", "")
        .replace("", "")
        .replace("", "")
        .replace("", "")
        .replace("yin", "淫")
        .replace("地~址~发~布~页~：、2·u·2·u·2·u、", "")
        .replace(
            "当前网址随时可能失效，请大家发送邮件到diyibanzhuＧｍａｉｌ．获取最新地址发布页！",
            "",
        )
}

fn main1() {
    let t = std::fs::read_to_string("/root/iepub/1.txt").unwrap();

    std::fs::write("2.txt", replace(t)).unwrap();
}

fn main() {
    use iepub::prelude::*;
    let mut builder =EpubBuilder::new().with_title("艳满人间都市录").with_creator("生活所迫")
    .with_description("周梦龙，一个才华横溢，帅气唇柔的青年，如愿的考入了公务员，进入系统以后，才发现，这里面一半以上是美女，熟女，少妇，少女，个个惊艳，人人妩媚，那么，主人公又如何在这都市之中混得风声水起，又如何的和这些美女们发现感情上的纠葛，请大家试目以待红尘都市……")
    .auto_gen_cover(true)
    .with_font("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc");

    let chap_start = vec!["一", "二", "三", "四", "五", "六", "七", "八", "九", "十"];
    // let v = vec!["一"];

    let res = fs::read_to_string("/root/iepub/lib/2.txt").unwrap();

    let v = vec![(13100, 10), (8831, 9), (3876, 12)];

    let lines: Vec<_> = res.split("\n").collect();

    let mut chap = EpubHtml::default().with_file_name("8.xhtml");

    let mut data = String::new();
    for (index, ele) in lines.iter().enumerate() {
        if index < 7 {
            continue;
        }
        if index == 12116 {
            data.push_str(ele.trim());
            continue;
        }
        let c = ele.trim().chars().collect::<Vec<char>>();
        if chap_start.iter().any(|f| ele.trim().starts_with(f))
        // && (!ele.trim().starts_with("一边") )
        // && !ele.trim().starts_with("一双")
        // && !ele.trim().starts_with("一向")
        //     && !ele.trim().starts_with("一开始")
        // && !ele.trim().starts_with("一方")
        // && !ele.trim().starts_with("一股")
        // && !ele.trim().starts_with("一阵")
        // && !ele.trim().starts_with("一")
        // && !ele.trim().starts_with("一")
        // && !ele.trim().starts_with("一")
        {
            if index > 385 && c.len() > 1 {
                if !(c[1].to_string() == "百" || c[1].to_string() == "十") {
                    // println!("temp = {:?}",c);
                    data.push_str(ele.trim());
                    continue;
                }
            }
            if index <= 385
                && (ele.trim().starts_with("一边")
                    || ele.trim().starts_with("一阵")
                    || ele.trim().starts_with("一个")
                    || ele.trim().starts_with("一路")
                    || ele.trim().starts_with("一辆")
                    || ele.trim().starts_with("一股"))
            {
                data.push_str(ele.trim());
                continue;
            }

            if data.is_empty() {
                chap = chap
                    .with_file_name(format!("1-{}.xhtml", index).as_str())
                    .with_title(ele.trim());
                println!("1 line = {index} title={}", ele.trim(),);

                continue;
            } else {
                if let Some((_, skip)) = v.iter().find(|s| s.0 == index) {
                    println!("{} {}", index, ele.trim());
                    let (start_byte, _) = ele.trim().char_indices().nth(*skip).unwrap();

                    let t = &ele.trim()[..start_byte];
                    builder = builder
                        .add_chapter(chap.with_data(replace(data.clone()).as_bytes().to_vec()));
                    chap = EpubHtml::default()
                        .with_title(t)
                        .with_file_name(format!("3-{index}.xhtml").as_str());
                    data.clear();
                    data.push_str(&ele.trim()[start_byte..]);
                    println!("3 line = {index} title={} {t}", ele.trim(),);
                    continue;
                }

                builder =
                    builder.add_chapter(chap.with_data(replace(data.clone()).as_bytes().to_vec()));
                data.clear();
                chap = EpubHtml::default()
                    .with_title(ele.trim())
                    .with_file_name(format!("2-{}.xhtml", index).as_str());
                println!("2 line = {index} title={}", ele.trim(),);
            }
        } else {
            data.push_str(format!("<p>{}</p>", ele.trim()).as_str());
        }
    }
    builder = builder.add_chapter(chap.with_data(replace(data.clone()).as_bytes().to_vec()));

    // let book = builder.book().unwrap();
    // for ele in book.chapters() {
    //     println!("f={} {}", ele.file_name(), ele.title());
    // }
    // println!("{:?}",builder.book().unwrap().chapters());
    builder.file("out.epub").unwrap();
}
