use quote::{format_ident, quote};
use std::env;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //bindings and params
    {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut bindings_file = std::fs::File::create(&Path::new(&out_dir).join("binding.rs"))?;
        let mut params_file = std::fs::File::create(&Path::new(&out_dir).join("param.rs"))?;

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
            let mut access = Vec::new();
            let mut pget = Vec::new();
            let mut pset = Vec::new();
            let mut unbind = Vec::new();
            for v in variants.iter() {
                let i = format_ident!("{}", v.0);
                let t = format_ident!("{}", v.2);

                let g = format_ident!("{}Get", v.0);
                let s = format_ident!("{}Set", v.0);
                let gs = format_ident!("{}GetSet", v.0);

                access.push(quote! {
                    #g(Arc<dyn ParamBindingGet<#t>>),
                    #s(Arc<dyn ParamBindingSet<#t>>),
                    #gs { get: Arc<dyn ParamBindingGet<#t>>, set: Arc<dyn ParamBindingSet<#t>>}
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
                        #(#access),*
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
        }

        let mut param_type_name_get_variants = Vec::new();
        let mut param_type_name_set_variants = Vec::new();
        let mut binding_type_name_get_variants = Vec::new();
        let mut binding_type_name_set_variants = Vec::new();
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
            binding_type_name_get_variants.push(quote! {
                Get::#i(_) => #fname
            });
            param_type_name_set_variants.push(quote! {
                ParamSet::#i(_) => #fname
            });
            binding_type_name_set_variants.push(quote! {
                Set::#i(_) => #fname
            });

            let get_ident = format_ident!("as_{}_get", fname);
            let set_ident = format_ident!("as_{}_set", fname);
            let type_ident = format_ident!("{}", tname);

            binding_typed_getset.push(quote! {
                pub fn #get_ident(&self) -> Option<::std::sync::Arc<dyn ParamBindingGet<#type_ident>>> {
                    /*
                    match self.as_get() {
                        Some(Get::#i(o)) => Some(o.clone()),
                        _ => None,
                    }
                    */
                    None
                }
                pub fn #set_ident(&self) -> Option<std::sync::Arc<dyn ParamBindingSet<#type_ident>>> {
                    /*
                    match self.as_set() {
                        Some(Set::#i(o)) => Some(o.clone()),
                        _ => None,
                    }
                    */
                    None
                }
            });
        }

        bindings_file.write_all(
            quote! {
                impl Instance {

                    #(#binding_typed_getset)*

                    /*
                    fn as_get(&self) -> Option<&Get> {
                        match &self.binding {
                            Access::Get(m) => Some(m),
                            Access::Set(_) => None,
                            Access::GetSet(_) => None, //TODO
                        }
                    }
                    fn as_set(&self) -> Option<&Set> {
                        match &self.binding {
                            Access::Get(_) => None,
                            Access::Set(m) => Some(m),
                            Access::GetSet(_) => None, //TODO
                        }
                    }
                    */

                    pub fn data_type_name(&self) -> &'static str {
                        //XXX
                        &"TODO"
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;

        params_file.write_all(
            quote! {
                impl ParamAccess {
                    fn as_get(&self) -> Option<&ParamGet> {
                        match self {
                            Self::Get { get: g, .. } => Some(g),
                            Self::Set { .. } => None,
                        }
                    }

                    fn as_set(&self) -> Option<&ParamSet> {
                        match self {
                            Self::Get { .. } => None,
                            Self::Set { set: s, .. } => Some(s),
                        }
                    }

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
                        //XXX
                        &"TODO"
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
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
