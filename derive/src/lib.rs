extern crate proc_macro;

use std::str::FromStr;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};

///
///
/// 对于Option<String>类型的结构体成员，生成相关方法，支持嵌套
///
/// # Examples:
///
/// ```rust
/// /// 访问成员 self.info.k
/// option_string_method!(info,k)
/// /// 访问成员 self.k
/// option_string_method!(k)
/// ```
///
#[proc_macro]
pub fn option_string_method(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let v: Vec<&str> = s.split(',').collect();

    let m = r#"pub fn set_{method}(&mut self, {method}: &str) {
        if let Some( c) = &mut self.{prefix}{method} {
            c.clear();
            c.push_str({method});
        } else {
            self.{prefix}{method} = Some(String::from({method}));
        }
    }
    pub fn with_{method}(mut self, {method}: &str) ->Self {
        self.set_{method}({method});
        self
    }
    pub fn {method}(&self) -> Option<&str> {
        self.{prefix}{method}.as_ref().map(|x|x.as_str())
    }"#;
    if v.len() == 2 {
        let r = m
            .replace("{prefix}", format!("{}.", v[0].trim()).as_str().trim())
            .replace("{method}", v[1].trim());
        return r.parse().unwrap();
    } else if v.len() == 1 {
        return m
            .replace("{prefix}", "")
            .replace("{method}", v[0].trim())
            .parse()
            .unwrap();
    }
    "".parse().unwrap()
}

#[proc_macro_attribute]
pub fn epub_base(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    // attrs.get(0).
    let mut ann = String::new();
    let mut has_derive = false;
    for ele in attrs {
        if ele.path().is_ident("derive") {
            has_derive = true;
            ann.push_str("#[derive(derive::EpubBaseTrail");
            let nested = ele
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .unwrap();
            for meta in nested {
                match meta {
                    syn::Meta::Path(p) => {
                        if let Some(i) = p.get_ident() {
                            ann.push_str(",");
                            ann.push_str(i.to_string().as_str());
                        }
                    }
                    // #[repr(C)]
                    _ => {
                        panic!("unrecognized ");
                        //  return Err(Error::new_spanned(meta, "unrecognized repr"));
                    }
                }
            }
            ann.push_str(")]");
        } else {
            let mut temp = proc_macro2::TokenStream::new();
            ele.to_tokens(&mut temp);
            ann.push_str(temp.to_string().as_str());
        }
    }
    if !has_derive {
        ann.push_str("#[derive(derive::EpubBaseTrail)]");
    }

    let expanded = match data {
        syn::Data::Struct(data_struct) => {
            let s = &data_struct.fields;
            let fields = match s {
                syn::Fields::Named(fields_named) => fields_named.named.iter().map(|field| {
                    quote! {
                         #field
                    }
                }),
                _ => panic!("derive(EpubBaseTrail) only supports structs with named fields"),
            };
            let m = quote! {
                pub struct #ident {
                    id:String,
                    _file_name:String,
                    media_type:String,
                    /// 数据
                    _data: Option<Vec<u8>>,
                        /// 处于读模式
                    reader:Option<std::rc::Rc<std::cell::RefCell< Box<dyn crate::EpubReaderTrait>>>>,
                    #(#fields),*// 结尾必须有个逗号，否则虽然生成的字符串语法正确，但是还是会报错
                }
            };

            // 直接把 ann 写到 quote! 宏中会给 ann 用双引号包裹
            let out = format!("{} {}", ann, m.to_string());
            TokenStream::from_str(out.as_str()).unwrap()
        }
        _ => panic!("derive(EpubBaseTrail) only supports structs"),
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(EpubBaseTrail)]
pub fn deriver_epub_base(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let expanded = match data {
        syn::Data::Struct(_) => {
            quote! {
                impl common::EpubItem for #ident {
                    fn file_name(&self)->&str{
                        self._file_name.as_str()
                    }
                    fn set_file_name(&mut self,value: &str){
                        self._file_name.clear();
                        self._file_name.push_str(value);
                    }

                    fn id(&self)->&str{
                        self.id.as_str()
                    }
                    fn set_id(&mut self,id:&str){
                        self.id.clear();
                        self.id.push_str(id);
                    }

                    fn set_data(&mut self, data: Vec<u8>) {
                        // if let Some(d) = &mut self._data {
                        //     d.clear();
                        //     d.append(data);
                        // }else{
                            self._data = Some(data);
                        // }
                    }


                }

                impl #ident {
                    pub fn with_file_name(mut self,value:&str)->Self{
                        common::EpubItem::set_file_name(&mut self, value);
                        self
                    }

                    pub fn with_data(mut self, value:Vec<u8>)->Self{
                        common::EpubItem::set_data(&mut self,value);
                        self
                    }
                }
            }
        }
        _ => panic!("derive(EpubBaseTrail) only supports structs"),
    };

    TokenStream::from(expanded)
}
