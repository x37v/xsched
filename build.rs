use quote::{format_ident, quote};
use std::env;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let mut bindings_file = std::fs::File::create(&Path::new(&out_dir).join("binding.rs"))?;
    let mut params_file = std::fs::File::create(&Path::new(&out_dir).join("param.rs"))?;
    let mut oscquery_file = std::fs::File::create(&Path::new(&out_dir).join("oscquery.rs"))?;

    //(enum Varaiant Name, str for function naming, actual type name)
    let variants = [
        ("Bool", "bool", "bool"),
        ("U8", "u8", "u8"),
        ("USize", "usize", "usize"),
        ("ISize", "isize", "isize"),
        ("Float", "float", "Float"),
        ("ClockData", "clock_data", "ClockData"),
        ("TickResched", "tick_resched", "TickResched"),
        ("TickSched", "tick_sched", "TickSched"),
    ];

    //build out Get, Set, GetSet, ParamGet and ParamSet
    {
        let mut access_variants = Vec::new();
        let mut access_data_type_name = Vec::new();
        let mut access_access_name = Vec::new();

        let mut pget = Vec::new();
        let mut pset = Vec::new();
        let mut param_data_type_name = Vec::new();
        let mut unbind = Vec::new();

        for v in variants.iter() {
            let i = format_ident!("{}", v.0);
            let t = format_ident!("{}", v.2);

            let g = format_ident!("{}Get", v.0);
            let s = format_ident!("{}Set", v.0);
            let gs = format_ident!("{}GetSet", v.0);
            let tname = v.2;

            //build the variants
            access_variants.push(quote! {
                #g(Arc<dyn ParamBindingGet<#t>>),
                #s(Arc<dyn ParamBindingSet<#t>>),
                #gs { get: Arc<dyn ParamBindingGet<#t>>, set: Arc<dyn ParamBindingSet<#t>> }
            });
            //get the data type name
            access_data_type_name.push(quote! {
                Self::#g(..) | Self::#s(..) | Self::#gs { .. } => &#tname
            });
            param_data_type_name.push(quote! {
                    Self::Get{ get : ParamGet::#i(..), .. } | Self::Set { set: ParamSet::#i(..), .. } => &#tname
                });
            //get the access name
            access_access_name.push(quote! {
                Self::#g(..) => &"get",
                Self::#s(..) => &"set",
                Self::#gs { .. } => &"getset"
            });
            pget.push(quote! {
                #i(::std::sync::Arc<BindingSwapGet<#t>>)
            });
            pset.push(quote! {
                #i(::std::sync::Arc<BindingSwapSet<#t>>)
            });
            unbind.push(quote! {
                Self::#i(b) => {
                    b.unbind();
                }
            });
        }
        bindings_file.write_all(
            quote! {
                /// Operations/data and their access.
                pub enum Access {
                    #(#access_variants),*
                }
                impl Access {
                    pub fn access_name(&self) -> &'static str {
                        match self {
                            #(#access_access_name),*
                        }
                    }
                    pub fn data_type_name(&self) -> &'static str {
                        match self {
                            #(#access_data_type_name),*
                        }
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;
        params_file.write_all(
            quote! {
                /// Parameters that you can get values from.
                pub enum ParamGet {
                    #(#pget),*
                }
                /// Parameters that you can set to a value.
                pub enum ParamSet {
                    #(#pset),*
                }
                impl ParamGet {
                    //TODO transform and return output?
                    pub fn unbind(&self) {
                        match self {
                            #(#unbind),*
                        }
                    }
                }
                impl ParamSet {
                    //TODO transform and return output?
                    pub fn unbind(&self) {
                        match self {
                            #(#unbind),*
                        }
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;

        let mut param_type_name_get_variants = Vec::new();
        let mut param_type_name_set_variants = Vec::new();
        let mut binding_typed_getset = Vec::new();

        let mut try_bind_variants = Vec::new();
        for (var, fname, tname) in variants.iter() {
            let g = format_ident!("as_{}_get", fname);
            let s = format_ident!("as_{}_set", fname);
            let i = format_ident!("{}", var);
            try_bind_variants.push(quote! {
                ParamAccess::Get { get: ParamGet::#i(p), binding: b } => {
                    if let Some(g) = binding.#g() {
                        let mut l = b.lock();
                        p.bind(g);
                        l.replace(binding);
                        Ok(())
                    } else {
                        Err(BindingError::NoGet)
                    }
                }
                ParamAccess::Set { set: ParamSet::#i(p), binding: b } => {
                    if let Some(s) = binding.#s() {
                        let mut l = b.lock();
                        p.bind(s);
                        l.replace(binding);
                        Ok(())
                    } else {
                        Err(BindingError::NoSet)
                    }
                }
            });
            //build up get and set
            param_type_name_get_variants.push(quote! {
                ParamGet::#i(_) => #fname
            });
            param_type_name_set_variants.push(quote! {
                ParamSet::#i(_) => #fname
            });

            let get_ident = format_ident!("as_{}_get", fname);
            let set_ident = format_ident!("as_{}_set", fname);
            let type_ident = format_ident!("{}", tname);
            let g = format_ident!("{}Get", var);
            let s = format_ident!("{}Set", var);
            let gs = format_ident!("{}GetSet", var);

            binding_typed_getset.push(quote! {
                pub fn #get_ident(&self) -> Option<::std::sync::Arc<dyn ParamBindingGet<#type_ident>>> {
                    match &self.binding {
                        Access::#g(m) => Some(m.clone()),
                        Access::#gs { get: m, .. } => Some(m.clone()),
                        _ => None
                    }
                }
                pub fn #set_ident(&self) -> Option<std::sync::Arc<dyn ParamBindingSet<#type_ident>>> {
                    match &self.binding {
                        Access::#s(m) => Some(m.clone()),
                        Access::#gs { set: m, .. } => Some(m.clone()),
                        _ => None
                    }
                }
            });
        }

        bindings_file.write_all(
            quote! {
                impl Instance {
                    #(#binding_typed_getset)*
                }
            }
            .to_string()
            .as_bytes(),
        )?;

        params_file.write_all(
            quote! {
                impl ParamAccess {
                    fn binding(&self) -> &Mutex<Option<Arc<Instance>>> {
                        match self {
                            Self::Get { binding: b, .. } => b,
                            Self::Set { binding: b, .. } => b,
                        }
                    }

                    /// Get the uuid of the bound param, if there is a binding.
                    pub fn uuid(&self) -> Option<uuid::Uuid> {
                        self.binding().lock().as_deref().map(|b| b.uuid().clone())
                    }

                    pub fn data_type_name(&self) -> &'static str {
                        match self {
                        #(#param_data_type_name),*
                        }
                    }

                    /// attempt to bind.
                    pub fn try_bind(&self, binding: Arc<Instance>) -> Result<(), BindingError> {
                        let b = match self {
                            #(#try_bind_variants)*
                        };
                        b
                    }

                    ///Get a `&str` representing the type of access: `"get", "set" or "getset"`
                    pub fn access_name(&self) -> &str {
                        match self {
                            ParamAccess::Get{ .. } => "get",
                            ParamAccess::Set{ .. } => "set",
                        }
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;
    }

    {
        oscquery_file.write_all(
            quote! {
                impl OSCQueryHandler {
                    pub fn add_binding_value(&self, binding: &Arc<Instance>, handle: ::oscquery::root::NodeHandle) {
                        //TODO
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;
    }
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
