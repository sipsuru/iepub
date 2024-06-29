extern crate proc_macro;

use std::str::FromStr;

use proc_macro::TokenStream;
use  syn::{parse_macro_input, DeriveInput};
use quote::{quote, ToTokens};

#[proc_macro]
pub fn epub_method_option(input:TokenStream)->TokenStream{


    let m = r#"pub fn set_{input}(&mut self, {input}: &str) {
        if let Some( c) = &mut self.info.{input} {
            c.clear();
            c.push_str({input});
        } else {
            self.info.{input} = Some(String::from({input}));
        }
    }

    pub fn get_{input}(&self) -> Option<&str> {
        self.info.{input}.as_ref().map(|x|x.as_str())
    }"#;

    
    m.replace("{input}", input.to_string().as_str()).parse().unwrap()
}


#[proc_macro_attribute]
pub fn epub_base(_attr: TokenStream, input:TokenStream)->TokenStream{

    let DeriveInput { ident, data, attrs, .. } = parse_macro_input!(input);

    // attrs.get(0).
    let mut ann = String::new();
    let mut has_derive = false;
    for ele in attrs {
        if ele.path().is_ident("derive") {
            has_derive = true;
            ann.push_str("#[derive(derive::EpubBaseTrail");
            let nested = ele.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated).unwrap();
             for meta in nested {
                 match meta {
                    syn::Meta::Path(p)=>{
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
        }else  {
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
                    file_name:String,
                    media_type:String,
                    /// 数据
                    data: Option<Vec<u8>>,
                    #(#fields),*// 结尾必须有个逗号，否则虽然生成的字符串语法正确，但是还是会报错
                }
            };
            
            // 直接把 ann 写到 quote! 宏中会给 ann 用双引号包裹
            let out =format!("{} {}",ann,m.to_string());
            TokenStream::from_str(out.as_str()).unwrap()


        },
        _ => panic!("derive(EpubBaseTrail) only supports structs"),
    };
 
    TokenStream::from(expanded)


}

#[proc_macro_derive(EpubBaseTrail)]
pub fn deriver_epub_base(input:TokenStream) ->TokenStream{



    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
 
    let expanded = match data {
        syn::Data::Struct(_) => {


            quote! {
                impl common::EpubItem for #ident {
                    fn get_file_name(&self)->&str{
                        self.file_name.as_str()
                    }
                    fn set_file_name(&mut self,value: &str){
                        self.file_name.clear();
                        self.file_name.push_str(value);
                    }

                    fn get_id(&self)->&str{
                        self.id.as_str()
                    }
                    fn set_id(&mut self,id:&str){
                        self.id.clear();
                        self.id.push_str(id);
                    }

                    fn set_data(&mut self, data: &mut Vec<u8>) {
                        if let Some(d) = &mut self.data {
                            d.clear();
                            d.append(data);
                        }else{
                            let mut v= Vec::new();
                            v.append(data);
                            self.data = Some(v);
                        }
                
                        todo!()
                    }
                    fn get_data(&self) -> Option<&[u8]>{
                       self.data.as_ref().map(|f|f.as_slice())
                    }
                    

                }

                impl #ident {
                    pub fn file_name(mut self,value:&str)->Self{
                        common::EpubItem::set_file_name(&mut self, value);
                        self
                    }
                    
                    pub fn data(mut self, mut value:Vec<u8>)->Self{
                        common::EpubItem::set_data(&mut self,&mut value);
                        self
                    }
                }
            }
        },
        _ => panic!("derive(EpubBaseTrail) only supports structs"),
    };
 
    TokenStream::from(expanded)
}

